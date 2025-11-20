use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    path::PathBuf,
    time::Duration,
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    is_dir: bool,
    is_verilib_meta: bool,
    specified: bool,
}

#[derive(Serialize, Deserialize)]
struct MetaFile {
    #[serde(default)]
    specified: bool,
    #[serde(flatten)]
    other: serde_json::Value,
}

struct App {
    current_dir: PathBuf,
    items: Vec<FileEntry>,
    state: ListState,
    search_mode: bool,
    search_query: String,
    search_results: Vec<FileEntry>,
    root_dir: PathBuf,
}

impl App {
    fn new() -> Result<Self> {
        let root_dir = std::env::current_dir()?.join(".verilib");
        let mut app = Self {
            current_dir: root_dir.clone(),
            items: Vec::new(),
            state: ListState::default(),
            search_mode: false,
            search_query: String::new(),
            search_results: Vec::new(),
            root_dir,
        };
        app.refresh_items()?;
        Ok(app)
    }

    fn refresh_items(&mut self) -> Result<()> {
        self.items.clear();
        
        if self.current_dir != self.root_dir {
            self.items.push(FileEntry {
                path: self.current_dir.parent().unwrap().to_path_buf(),
                is_dir: true,
                is_verilib_meta: false,
                specified: false,
            });
        }

        if self.current_dir.exists() {
            let mut entries: Vec<_> = fs::read_dir(&self.current_dir)?
                .filter_map(|e| e.ok())
                .collect();
            
            entries.sort_by_key(|e| e.path());

            for entry in entries {
                let path = entry.path();
                let is_dir = path.is_dir();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                
                if !is_dir && !file_name.contains(".meta.") {
                    continue;
                }

                let is_verilib_meta = path.extension().map_or(false, |ext| ext == "verilib");
                
                let mut specified = false;
                if is_verilib_meta {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(meta) = serde_json::from_str::<MetaFile>(&content) {
                            specified = meta.specified;
                        }
                    }
                }

                self.items.push(FileEntry {
                    path,
                    is_dir,
                    is_verilib_meta,
                    specified,
                });
            }
        }
        
        if self.state.selected().is_none() && !self.items.is_empty() {
            self.state.select(Some(0));
        }
        
        Ok(())
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        if self.search_query.is_empty() {
            return;
        }

        let walker = WalkDir::new(&self.root_dir).into_iter();
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            
            if path.is_file() && file_name.contains(".meta.") && path.to_string_lossy().contains(&self.search_query) {
                let is_verilib_meta = path.extension().map_or(false, |ext| ext == "verilib");
                let mut specified = false;
                
                if is_verilib_meta {
                    if let Ok(content) = fs::read_to_string(path) {
                        if let Ok(meta) = serde_json::from_str::<MetaFile>(&content) {
                            specified = meta.specified;
                        }
                    }
                }

                self.search_results.push(FileEntry {
                    path: path.to_path_buf(),
                    is_dir: false,
                    is_verilib_meta,
                    specified,
                });
            }
        }
        
        if !self.search_results.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }

    fn toggle_current(&mut self) -> Result<()> {
        let items = if self.search_mode {
            &mut self.search_results
        } else {
            &mut self.items
        };

        if let Some(selected) = self.state.selected() {
            if let Some(item) = items.get_mut(selected) {
                if item.is_verilib_meta {
                    let content = fs::read_to_string(&item.path)?;
                    let mut meta: MetaFile = serde_json::from_str(&content)?;
                    meta.specified = !meta.specified;
                    item.specified = meta.specified;
                    
                    let new_content = serde_json::to_string_pretty(&meta)?;
                    fs::write(&item.path, new_content)?;
                }
            }
        }
        Ok(())
    }

    fn enter_directory(&mut self) -> Result<()> {
        if self.search_mode {
            return Ok(());
        }

        if let Some(selected) = self.state.selected() {
            if let Some(item) = self.items.get(selected) {
                if item.is_dir {
                    if item.path == self.current_dir.parent().unwrap_or(&self.current_dir).to_path_buf() 
                       && self.current_dir != self.root_dir {
                        self.current_dir = item.path.clone();
                    } else {
                        self.current_dir = item.path.clone();
                    }
                    self.state.select(Some(0));
                    self.refresh_items()?;
                }
            }
        }
        Ok(())
    }

    fn go_up(&mut self) -> Result<()> {
        if self.search_mode {
            self.search_mode = false;
            self.search_query.clear();
            self.state.select(Some(0));
            return Ok(());
        }

        if self.current_dir != self.root_dir {
            if let Some(parent) = self.current_dir.parent() {
                self.current_dir = parent.to_path_buf();
                self.state.select(Some(0));
                self.refresh_items()?;
            }
        }
        Ok(())
    }

    fn next(&mut self) {
        let count = if self.search_mode {
            self.search_results.len()
        } else {
            self.items.len()
        };

        if count == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let count = if self.search_mode {
            self.search_results.len()
        } else {
            self.items.len()
        };

        if count == 0 {
            return;
        }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub async fn handle_status_update() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = App::new();
    
    if let Err(e) = app_result {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        return Err(e);
    }

    let mut app = app_result.unwrap();

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.search_mode {
                        match key.code {
                            KeyCode::Esc => {
                                app.search_mode = false;
                                app.search_query.clear();
                                app.state.select(Some(0));
                            }
                            KeyCode::Enter => {
                                app.toggle_current()?;
                            }
                            KeyCode::Tab => {
                                app.toggle_current()?;
                            }
                            KeyCode::Char(c) => {
                                app.search_query.push(c);
                                app.perform_search();
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.perform_search();
                            }
                            KeyCode::Up => app.previous(),
                            KeyCode::Down => app.next(),
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('/') => {
                                app.search_mode = true;
                                app.search_query.clear();
                                app.perform_search();
                            }
                            KeyCode::Left | KeyCode::Char('h') => app.go_up()?,
                            KeyCode::Right | KeyCode::Char('l') => app.enter_directory()?,
                            KeyCode::Enter => {
                                let is_dir = app.state.selected()
                                    .and_then(|i| app.items.get(i))
                                    .map(|i| i.is_dir)
                                    .unwrap_or(false);
                                if is_dir {
                                    app.enter_directory()?;
                                } else {
                                    app.toggle_current()?;
                                }
                            }
                            KeyCode::Tab => app.toggle_current()?,
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Char(' ') => app.toggle_current()?,
                            KeyCode::Esc => return Ok(()),
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(format!(" Verilib Validation Manager - {} ", app.current_dir.display()))
        .block(Block::default().borders(Borders::ALL).title("Location"));
    f.render_widget(title, chunks[0]);

    let is_search = app.search_mode;
    let parent_dir = app.current_dir.parent().unwrap_or(&app.current_dir).to_path_buf();

    let items: Vec<ListItem> = if is_search {
        app.search_results.iter()
    } else {
        app.items.iter()
    }
    .map(|i| {
        let name = if i.is_dir && i.path == parent_dir {
            "..".to_string()
        } else if is_search {
            let path_str = i.path.to_string_lossy();
            let relative_path = if let Some(idx) = path_str.find(".verilib/") {
                path_str[idx + 9..].to_string()
            } else {
                path_str.to_string()
            };
            let re = Regex::new(r"\[\d+\] - ").unwrap();
            re.replace_all(&relative_path, "").to_string().replace(".meta.verilib", "")
        } else {
            let raw_name = i.path.file_name().unwrap_or_default().to_string_lossy();
            let re = Regex::new(r"^\[\d+\] - ").unwrap();
            re.replace(&raw_name, "").to_string().replace(".meta.verilib", "")
        };

        let (icon, style) = if i.is_dir {
            ("📁", Style::default())
        } else {
            if i.specified {
                ("✅", Style::default())
            } else {
                ("[]", Style::default())
            }
        };

        let content = Line::from(vec![
            Span::styled(format!("{} ", icon), style),
            Span::raw(name),
        ]);
        ListItem::new(content)
    })
    .collect();

    let list_title = if app.search_mode {
        "Search Results"
    } else {
        "Files"
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], &mut app.state);

    if app.search_mode {
        let search = Paragraph::new(format!("Search: {}", app.search_query))
            .block(Block::default().borders(Borders::ALL).title("Search Mode (Tab/Enter to toggle, Esc to exit)"));
        f.render_widget(search, chunks[2]);
    } else {
        let help = Paragraph::new("Nav: ↑/↓/←/→ | Toggle: Space/Tab/Enter | Search: / | Quit: Esc")
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[2]);
    }
}
