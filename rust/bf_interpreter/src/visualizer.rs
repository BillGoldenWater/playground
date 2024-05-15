use std::{
    io::{stdout, Stdout},
    path::Path,
    str::FromStr,
    time::{Duration, Instant},
    usize,
};

use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand as _,
};
use functional_utils::FunctionalUtils;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Styled, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
    Frame, Terminal,
};

use self::interpreter_state::InterpreterState;
use crate::interpreter::Interpreter;
pub mod interpreter_state;

#[derive(Debug)]
pub struct Visualizer {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,

    interpreter: Option<(Interpreter, InterpreterState)>,
    speed: u64,

    input_buffer: String,
}

impl Visualizer {
    pub fn init() -> anyhow::Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;
        stdout()
            .execute(EnterAlternateScreen)
            .context("failed to enter alternate screen")?;

        Self {
            terminal: Terminal::new(CrosstermBackend::new(stdout()))
                .context("failed to create terminal")?
                .some(),

            interpreter: None,
            speed: 1,

            input_buffer: String::new(),
        }
        .into_ok()
    }

    pub fn unload(self) -> anyhow::Result<()> {
        disable_raw_mode().context("failed to disable raw mode")?;
        stdout()
            .execute(LeaveAlternateScreen)
            .context("failed to leave alternate screen")?;
        Ok(())
    }

    /// Return: should exit
    pub fn tick(&mut self) -> anyhow::Result<bool> {
        if self.handle_event().context("failed to handle event")? {
            return Ok(true);
        }

        let mut terminal = self.terminal.take().unwrap();
        terminal
            .draw(|frame| self.render(frame))
            .context("failed to draw")?;
        self.terminal = Some(terminal);

        match &mut self.interpreter {
            Some((i, state @ InterpreterState::Running)) => {
                for _ in 0..self.speed {
                    if i.tick() {
                        *state = InterpreterState::Paused;
                    }
                }
            }
            _ => {}
        }

        Ok(false)
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .split(frame.size());

        self.render_interpreter(frame, layout[0]);
        self.render_command_input(frame, layout[1]);
    }

    fn render_interpreter(&self, frame: &mut Frame, rect: Rect) {
        match self.interpreter.as_ref() {
            None => frame.render_widget(
                Paragraph::new("no interpreter")
                    .block(Block::default().padding(Padding::top(rect.height / 2)))
                    .centered(),
                rect,
            ),
            Some((i, running)) => self.render_interpreter_inner(frame, rect, i, *running),
        }
    }

    fn render_interpreter_inner(
        &self,
        frame: &mut Frame,
        rect: Rect,
        interpreter: &Interpreter,
        running: InterpreterState,
    ) {
        // region warpping output
        let output = String::from_utf8_lossy(&interpreter.output);
        let mut line_buf = String::new();
        let mut output_lines = vec![];

        for ch in output.chars() {
            match ch {
                '\n' => {
                    output_lines.push(line_buf.clone());
                    line_buf.clear()
                }
                _ => {
                    line_buf.push(ch);

                    if line_buf.len() >= rect.width as usize {
                        output_lines.push(line_buf.clone());
                        line_buf.clear()
                    }
                }
            }
        }

        if !line_buf.is_empty() {
            output_lines.push(line_buf);
        }
        // endregion

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(10.min((rect.height / 2).into())),
            ])
            .split(rect);
        // TODO: memory and instructions

        self.render_interpreter_output(frame, layout[1], output_lines);
    }

    fn render_interpreter_output(&self, frame: &mut Frame, rect: Rect, output: Vec<String>) {
        let output_len = output.len();
        output
            .into_iter()
            .skip(output_len.saturating_sub(rect.height as usize))
            .map(|line| Line::from(line))
            .collect::<Vec<_>>()
            .then(|lines| frame.render_widget(Paragraph::new(lines), rect));
    }

    fn render_command_input(&self, frame: &mut Frame, rect: Rect) {
        let indicator_style = if let Some(true) = self
            .interpreter
            .as_ref()
            .map(|(i, state)| i.waitting_input && state.is_running())
        {
            Style::default().blue()
        } else {
            Style::default()
        };

        let extra_len = self
            .input_buffer
            .len()
            .saturating_sub((rect.width as usize).saturating_sub(2));

        let input_buf = if extra_len > 0 {
            Span::from(
                self.input_buffer
                    .chars()
                    .skip(extra_len)
                    .collect::<String>(),
            )
        } else {
            Span::from(&self.input_buffer)
        };

        let line = Line::from(vec![">".set_style(indicator_style), input_buf]);
        frame.render_widget(Paragraph::new(line), rect);
    }

    /// Return: should exit
    fn handle_event(&mut self) -> anyhow::Result<bool> {
        while event::poll(Duration::from_millis(1)).context("failed to pool event")? {
            let event = event::read().context("failed to read event")?;
            match event {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        if key.modifiers == KeyModifiers::CONTROL {
                            if key.code == KeyCode::Char('c') {
                                return Ok(true);
                            }
                        } else {
                            match key.code {
                                KeyCode::Backspace => {
                                    self.input_buffer.pop();
                                }
                                KeyCode::Enter => {
                                    self.handle_input();
                                }
                                KeyCode::Char(ch) => {
                                    self.input_buffer.push(ch);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => return Ok(false),
            }
        }

        Ok(false)
    }

    fn handle_input(&mut self) {
        let buffer: Box<str> = Box::from(self.input_buffer.as_str());
        self.input_buffer.clear();

        if let Some(command) = buffer.strip_prefix('/') {
            let mut command = command.split(' ');
            let name = command.next();
            if let Some(name) = name {
                self.handle_command(name, command);
            } else {
                // TODO:
            }
        } else {
            if let Some((interpreter, _)) = self.interpreter.as_mut() {
                interpreter.input_buf.extend(buffer.bytes());
            } else {
                // TODO:
            }
        }
    }

    fn handle_command<'item>(
        &mut self,
        name: &'item str,
        mut args: impl Iterator<Item = &'item str>,
    ) {
        match name {
            "load" => {
                let code = args.collect::<Vec<_>>().join(" ");
                if code.is_empty() {
                    // TODO:
                    return;
                }

                match Interpreter::from_str(&code) {
                    Ok(interpreter) => {
                        self.interpreter = Some((interpreter, InterpreterState::Paused))
                    }
                    Err(_err) => {
                        // TODO:
                    }
                }
            }
            "load_file" => {
                let path = args.collect::<Vec<_>>().join(" ");
                if path.is_empty() {
                    // TODO:
                    return;
                }

                match Interpreter::from_file(Path::new(&path)) {
                    Ok(interpreter) => {
                        self.interpreter = Some((interpreter, InterpreterState::Paused))
                    }
                    Err(_err) => {
                        // TODO:
                    }
                }
            }
            "run" => {
                if let Some((_, running)) = &mut self.interpreter {
                    *running = InterpreterState::Running;
                }
            }
            "pause" => {
                if let Some((_, running)) = &mut self.interpreter {
                    *running = InterpreterState::Paused;
                }
            }
            "speed" => {
                if let Some(speed) = args.next() {
                    if let Ok(speed) = speed.parse::<u64>() {
                        self.speed = speed
                    }
                }
            }
            _ => {
                // TODO:
            }
        }
    }
}
