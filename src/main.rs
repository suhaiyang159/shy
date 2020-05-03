#[macro_use]
mod color;
mod ssh_config;

use ssh_config::{load_ssh_config, HostMap};

use std::{
    io::{self, Stdout, Write},
    os::unix::process::CommandExt,
    panic,
    process::Command,
};
use termion::{
    clear::{All as ClearAll, CurrentLine as ClearLine},
    cursor::{Goto, Hide as HideCursor, Show as ShowCursor},
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
    screen::{ToAlternateScreen, ToMainScreen},
    terminal_size,
};

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Search,
    Navigate,
}

fn main() -> Result<(), io::Error> {
    if let Some(hostname) = run()? {
        std::env::set_var("TERM", "xterm");
        let mut cmd = Command::new("ssh");
        let cmd = cmd.arg(hostname);
        let err = cmd.exec();
        eprintln!("{:?}", err);
    }

    Ok(())
}

fn run() -> Result<Option<String>, io::Error> {
    let hosts = load_ssh_config()?;
    let mut stdout = setup_terminal()?;
    setup_panic_hook();

    let mut selected = 0;
    let mut mode = InputMode::Navigate;
    let mut input = String::new();

    update()?;
    draw(&hosts, selected, "")?;

    while let Some(Ok(event)) = io::stdin().keys().next() {
        write!(stdout, "{}{}event: {:?}", Goto(1, 7), ClearLine, event)?;
        stdout.flush()?;

        match mode {
            InputMode::Navigate => match event {
                Key::Char('q') | Key::Ctrl('c') => break,
                Key::Char('i') | Key::Char('s') => mode = InputMode::Search,
                Key::Up | Key::Ctrl('p') => {
                    if selected == 0 {
                        selected = hosts.len() - 1;
                    } else {
                        selected -= 1;
                    }
                }
                Key::Down | Key::Ctrl('n') => {
                    if selected >= hosts.len() - 1 {
                        selected = 0;
                    } else {
                        selected += 1;
                    }
                }
                Key::Char('\n') => {
                    if let Some(host) = hosts.iter().nth(selected) {
                        shutdown_terminal()?;
                        return Ok(Some(host.0.clone()));
                    } else {
                        panic!("can't find host");
                    }
                }
                _ => {}
            },
            InputMode::Search => match event {
                Key::Ctrl('c') | Key::Esc => {
                    input.clear();
                    mode = InputMode::Navigate;
                }
                Key::Backspace => {
                    if !input.is_empty() {
                        input.truncate(input.len() - 1);
                    }
                }
                Key::Char('\n') => {
                    if let Some(host) = hosts.iter().nth(selected) {
                        shutdown_terminal()?;
                        return Ok(Some(host.0.clone()));
                    } else {
                        panic!("can't find host");
                    }
                }
                Key::Char(c) => {
                    input.push(c);
                }
                _ => {}
            },
        }

        draw(&hosts, selected, &input)?;
    }

    shutdown_terminal()?;
    Ok(None)
}

/// Switch to alternate mode, set colors, hide cursor.
fn setup_terminal() -> Result<RawTerminal<Stdout>, io::Error> {
    let mut stdout = io::stdout().into_raw_mode()?;
    write!(stdout, "{}", ToAlternateScreen)?;
    write!(stdout, "{}", HideCursor)?;
    write!(stdout, "{}", ClearAll)?;
    write!(stdout, "{}", Goto(1, 1))?;
    stdout.flush()?;
    Ok(stdout)
}

/// Restore terminal state to pre-launch.
fn shutdown_terminal() -> Result<(), io::Error> {
    let stdout = io::stdout();
    stdout.into_raw_mode()?.suspend_raw_mode()?;
    let mut stdout = io::stdout();
    write!(stdout, "{}", ShowCursor)?;
    write!(stdout, "{}", ToMainScreen)?;
    stdout.flush()?;
    Ok(())
}

/// We need to cleanup the terminal before exiting, even on panic!
fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let _ = shutdown_terminal();
        println!("{}", panic_info);
    }));
}

/// Update our state in response to key presses.
fn update() -> Result<(), io::Error> {
    Ok(())
}

/// Draw the app.
fn draw(hosts: &HostMap, selected: usize, input: &str) -> Result<(), io::Error> {
    let (_cols, rows) = terminal_size()?;
    let mut stdout = io::stdout();

    let prompt = format!(
        "{}{}{}{}",
        Goto(1, rows - 2),
        ClearLine,
        color_string!(">> ", Bold, White),
        input
    );

    write!(
        stdout,
        "{}{}{}{}{}{}{}",
        ClearAll,
        prompt,
        Goto(1, rows - 1),
        color!(MagentaBG),
        color!(Yellow),
        ClearLine,
        color_string!("shy", MagentaBG, Yellow, Bold)
    )?;

    let mut row = 3;
    for (i, (host, _config)) in hosts.iter().enumerate() {
        write!(
            stdout,
            "{}{}",
            Goto(1, row),
            if i == selected {
                format!("> {}", color_string!(host, Yellow, Bold))
            } else {
                format!("  {}", color_string!(host, White))
            }
        )?;
        row += 1;
    }

    stdout.flush()?;
    Ok(())
}
