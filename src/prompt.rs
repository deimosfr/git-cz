use promkit::{
    Prompt, Signal, TerminalModes, TerminalSession,
    core::{
        Widget,
        crossterm::{
            event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
            style::{Attribute, Attributes, Color, ContentStyle},
        },
        render::{Renderer, SharedRenderer},
    },
    widgets::{
        listbox::{self, Listbox},
        text::{self, Text},
        text_editor::{self, Mode},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PickerIndex {
    Title,
    Query,
    List,
}

struct Picker {
    renderer: Option<SharedRenderer<PickerIndex>>,
    title: text::State,
    query: text_editor::State,
    items: Vec<String>,
    list: listbox::State,
}

impl Picker {
    fn new(title: &str, items: &[String], lines: usize) -> Self {
        Self {
            renderer: None,
            title: title_state(title),
            query: editor_state(),
            items: items.to_vec(),
            list: listbox::State {
                listbox: Listbox::from(items),
                config: listbox::Config {
                    cursor: "❯ ".into(),
                    active_item_style: Some(ContentStyle {
                        foreground_color: Some(Color::DarkCyan),
                        ..Default::default()
                    }),
                    inactive_item_style: Some(ContentStyle::default()),
                    lines: Some(lines),
                },
            },
        }
    }

    async fn render(&self) -> promkit::anyhow::Result<()> {
        self.renderer
            .as_ref()
            .ok_or_else(|| promkit::anyhow::anyhow!("Renderer not initialized"))?
            .update([
                (PickerIndex::Title, self.title.create_graphemes()),
                (PickerIndex::Query, self.query.create_graphemes()),
                (PickerIndex::List, self.list.create_graphemes()),
            ])
            .render()
            .await
    }
}

#[promkit::async_trait::async_trait]
impl Prompt for Picker {
    async fn initialize(&mut self) -> promkit::anyhow::Result<()> {
        self.renderer = Some(SharedRenderer::new(
            Renderer::try_new_with_graphemes(
                [
                    (PickerIndex::Title, self.title.create_graphemes()),
                    (PickerIndex::Query, self.query.create_graphemes()),
                    (PickerIndex::List, self.list.create_graphemes()),
                ],
                true,
            )
            .await?,
        ));
        Ok(())
    }

    async fn evaluate(&mut self, event: &Event) -> promkit::anyhow::Result<Signal> {
        let signal = match event {
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }) if !self.list.listbox.is_empty() => Signal::Quit,
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }) => return Err(promkit::anyhow::anyhow!("ctrl+c")),
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }) => {
                self.list.listbox.backward();
                Signal::Continue
            }
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }) => {
                self.list.listbox.forward();
                Signal::Continue
            }
            Event::Key(key)
                if key.kind == KeyEventKind::Press && key.state == KeyEventState::NONE =>
            {
                let before = self.query.texteditor.text_without_cursor().to_string();
                edit(&mut self.query, key);
                let query = self.query.texteditor.text_without_cursor().to_string();
                if query != before {
                    let query = query.to_lowercase();
                    self.list.listbox = Listbox::from(
                        self.items
                            .iter()
                            .filter(|item| item.to_lowercase().contains(&query)),
                    );
                }
                Signal::Continue
            }
            _ => Signal::Continue,
        };
        self.render().await?;
        Ok(signal)
    }

    type Return = String;

    fn finalize(&mut self) -> promkit::anyhow::Result<Self::Return> {
        Ok(self.list.listbox.get().to_string())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum InputIndex {
    Title,
    Input,
    Error,
}

struct Input {
    renderer: Option<SharedRenderer<InputIndex>>,
    title: text::State,
    input: text_editor::State,
    error: text::State,
    validator: Option<fn(&str) -> bool>,
    validation_message: &'static str,
}

impl Input {
    fn new(
        title: &str,
        validator: Option<fn(&str) -> bool>,
        validation_message: &'static str,
    ) -> Self {
        Self {
            renderer: None,
            title: title_state(title),
            input: editor_state(),
            error: text::State {
                config: text::Config {
                    style: Some(ContentStyle {
                        foreground_color: Some(Color::DarkRed),
                        attributes: Attributes::from(Attribute::Bold),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            },
            validator,
            validation_message,
        }
    }

    async fn render(&self) -> promkit::anyhow::Result<()> {
        self.renderer
            .as_ref()
            .ok_or_else(|| promkit::anyhow::anyhow!("Renderer not initialized"))?
            .update([
                (InputIndex::Title, self.title.create_graphemes()),
                (InputIndex::Input, self.input.create_graphemes()),
                (InputIndex::Error, self.error.create_graphemes()),
            ])
            .render()
            .await
    }
}

#[promkit::async_trait::async_trait]
impl Prompt for Input {
    async fn initialize(&mut self) -> promkit::anyhow::Result<()> {
        self.renderer = Some(SharedRenderer::new(
            Renderer::try_new_with_graphemes(
                [
                    (InputIndex::Title, self.title.create_graphemes()),
                    (InputIndex::Input, self.input.create_graphemes()),
                    (InputIndex::Error, self.error.create_graphemes()),
                ],
                true,
            )
            .await?,
        ));
        Ok(())
    }

    async fn evaluate(&mut self, event: &Event) -> promkit::anyhow::Result<Signal> {
        let signal = match event {
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }) => {
                let value = self.input.texteditor.text_without_cursor().to_string();
                if self.validator.is_none_or(|validator| validator(&value)) {
                    Signal::Quit
                } else {
                    self.error.text = Text::from(self.validation_message);
                    Signal::Continue
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            }) => return Err(promkit::anyhow::anyhow!("ctrl+c")),
            Event::Key(key)
                if key.kind == KeyEventKind::Press && key.state == KeyEventState::NONE =>
            {
                edit(&mut self.input, key);
                Signal::Continue
            }
            _ => Signal::Continue,
        };
        self.render().await?;
        Ok(signal)
    }

    type Return = String;

    fn finalize(&mut self) -> promkit::anyhow::Result<Self::Return> {
        Ok(self.input.texteditor.text_without_cursor().to_string())
    }
}

fn title_state(title: &str) -> text::State {
    text::State {
        text: Text::from(title),
        config: text::Config {
            style: Some(ContentStyle {
                attributes: Attributes::from(Attribute::Bold),
                ..Default::default()
            }),
            ..Default::default()
        },
    }
}

fn editor_state() -> text_editor::State {
    text_editor::State {
        config: text_editor::Config {
            prefix: "❯❯ ".into(),
            prefix_style: ContentStyle {
                foreground_color: Some(Color::DarkGreen),
                ..Default::default()
            },
            active_char_style: ContentStyle {
                background_color: Some(Color::DarkCyan),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

fn edit(input: &mut text_editor::State, key: &KeyEvent) {
    match (key.code, key.modifiers) {
        (KeyCode::Left, KeyModifiers::NONE) => {
            input.texteditor.backward();
        }
        (KeyCode::Right, KeyModifiers::NONE) => {
            input.texteditor.forward();
        }
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => input.texteditor.move_to_head(),
        (KeyCode::Char('e'), KeyModifiers::CONTROL) => input.texteditor.move_to_tail(),
        (KeyCode::Backspace, KeyModifiers::NONE) => input.texteditor.erase(),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => input.texteditor.erase_all(),
        (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            match input.config.edit_mode {
                Mode::Insert => input.texteditor.insert(ch),
                Mode::Overwrite => input.texteditor.overwrite(ch),
            }
        }
        _ => {}
    }
}

pub async fn select(
    title: &str,
    choices: &[String],
    lines: usize,
) -> promkit::anyhow::Result<String> {
    let _session =
        TerminalSession::try_new(TerminalModes::RAW_MODE | TerminalModes::HIDDEN_CURSOR)?;
    Picker::new(title, choices, lines).run().await
}

pub async fn input(title: &str) -> promkit::anyhow::Result<String> {
    let _session =
        TerminalSession::try_new(TerminalModes::RAW_MODE | TerminalModes::HIDDEN_CURSOR)?;
    Input::new(title, None, "").run().await
}

pub async fn integer(title: &str) -> promkit::anyhow::Result<String> {
    let _session =
        TerminalSession::try_new(TerminalModes::RAW_MODE | TerminalModes::HIDDEN_CURSOR)?;
    Input::new(
        title,
        Some(|value| value.trim().parse::<i32>().is_ok()),
        "Enter a valid integer.",
    )
    .run()
    .await
}
