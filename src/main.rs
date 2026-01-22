//version 1.0 of TO_DO
use std::io::{stdout, Stdout};
use anyhow::Result;
use chrono::{DateTime, Local, Datelike, NaiveDate};
use ratatui::text::{Line, Span};
use::serde::{Serialize, Deserialize};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Modifier, Style},
    layout::{Alignment, Constraint, Direction, Layout },
    widgets::{Block, Borders, List, ListItem, ListState,Paragraph },
    Frame,
    Terminal,
};

#[derive(Serialize, Deserialize, Clone)]
struct Task{
    event_name: String,
    completed: bool,
    deadline: Option<DateTime<Local>>,
    priority: u8,
}

enum Focus{
    TaskList,
    CalendarList,
    TaskDetail,
    NewTaskCreation,
}
struct App{
    tasks: Vec<Task>,
    should_quit: bool,
    list_state: ListState,
    current_date: DateTime<Local>,
    focus: Focus,
    calendar_curosr_day: u32, //u32 for chrono use cuz it uses u32 and casting u8 non stop would be a burden not worth saving a few bytes
    input_buffer: String,
    input_priority: u8,
}

impl App {
    fn next_task(&mut self){
        let key_selected = match self.list_state.selected(){
            Some(key_selected) => {
                if key_selected >= self.tasks.len() - 1{
                    0
                }else{
                    key_selected + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(key_selected));
    }


    fn previous_task(&mut self){
        let key_selected = match self.list_state.selected(){
            Some(key_selected) => {
                if key_selected == 0{
                    self.tasks.len() - 1 //we're at the top and we go the the bottom
                }else {
                    key_selected - 1 //we're somewhere other than the top and we go one up
                }
            }
            None => 0,
        };
        self.list_state.select(Some(key_selected));
    }

    fn days_in_current_month(&mut self) -> u32{
        let year = self.current_date.year();
        let month = self.current_date.month();
        let next_month_date = NaiveDate::from_ymd_opt(
            match month {
                12 => year + 1,
                _ => year,
            },
            match month {
                12 => 1,
                _ => month + 1,
            },
            1,
        ).unwrap();

        let current_month_days =  NaiveDate::from_ymd_opt(year, month, 1).unwrap();

        next_month_date.signed_duration_since(current_month_days).num_days() as u32
    }

    fn next_day(&mut self){
        let max_days = self.days_in_current_month();
        self.calendar_curosr_day += 1;
        self.current_date = self.current_date.with_day(1).unwrap(); //without this there were some problems
        let current_month = self.current_date.month();
        if self.calendar_curosr_day > max_days {
                self.calendar_curosr_day = 1;
            if current_month == 12 {
                let next_year = self.current_date.year() + 1;
                self.current_date =self.current_date.with_year(next_year).unwrap().with_month(1).unwrap();
            }else {
                self.current_date =self.current_date.with_month(current_month+1).unwrap();


            }
        }
    }
    fn next_week(&mut self){
        let max_days = self.days_in_current_month();
        self.calendar_curosr_day += 7;
        self.current_date = self.current_date.with_day(1).unwrap();
        let current_month = self.current_date.month();
        if self.calendar_curosr_day > max_days {
            self.calendar_curosr_day -= max_days;
            if current_month == 12 {
                let next_year = self.current_date.year() + 1;
                self.current_date =self.current_date.with_year(next_year).unwrap().with_month(1).unwrap();
            }else {
                self.current_date =self.current_date.with_month(current_month+1).unwrap();
            }
        }
    }

    fn previous_day(&mut self){
        self.calendar_curosr_day -= 1;
        if self.calendar_curosr_day < 1 {
            self.current_date = self.current_date.with_day(1).unwrap();
            let current_month = self.current_date.month();
            if current_month == 1 {
                let previous_year = self.current_date.year() - 1;
                self.current_date =self.current_date.with_year(previous_year).unwrap().with_month(12).unwrap();
            }else {
                self.current_date =self.current_date.with_month(current_month-1).unwrap();
            }

            let new_max_days = self.days_in_current_month();
            self.calendar_curosr_day = new_max_days;
        }
    }
    fn previous_week(&mut self){
        let minus_week = (self.calendar_curosr_day as i32) - 7;
        if minus_week < 1 {
            self.current_date = self.current_date.with_day(1).unwrap();
            let current_month = self.current_date.month();
            if current_month == 1 {
                let previous_year = self.current_date.year() -1;
                self.current_date =self.current_date.with_year(previous_year).unwrap().with_month(12).unwrap();
            }else{
                self.current_date =self.current_date.with_month(current_month-1).unwrap();
            }
            let new_max_days = self.days_in_current_month() as i32;
            self.calendar_curosr_day = (new_max_days + minus_week) as u32;
        }else{
            self.calendar_curosr_day = minus_week as u32;
        }
    }

    fn delete_task(&mut self) {
        if let Some(selected_index) = self.list_state.selected() {

            self.tasks.remove(selected_index);

            //FIXING THE CURSOR
            if self.tasks.is_empty() {
                //case A: List is now empty -> Select nothing
                self.list_state.select(None);
            } else if selected_index >= self.tasks.len() {
                //case B: We deleted the last item -> Move cursor UP by one
                self.list_state.select(Some(selected_index - 1));
            }
            //case C: we deleted a middle
        }
    }
    fn export(&self)->Result<()> {
        let json_data = serde_json::to_string_pretty(&self.tasks)?;
        std::fs::write("tasks.json", json_data)?;
        Ok(())
    }

    fn load()->Result<Vec<Task>> {
        match std::fs::read_to_string("tasks.json") {
            Ok(json_content) => {
                let loaded_task: Vec<Task> = serde_json::from_str(&json_content)?;
                Ok(loaded_task)
            }
            Err(_) => {
                Ok(Vec::new())
            }
        }
    }
}


fn main() -> Result<()> {
    let mut terminal = init_terminal()?;
    let saved_tasks = App::load().unwrap_or_else(|_| Vec::new());
    let mut app = App{ should_quit: false, list_state: ListState::default(), current_date: Local::now(), focus: Focus::TaskList, calendar_curosr_day: Local::now().day(), input_buffer: String::new(), input_priority: 1, tasks: saved_tasks, //test V1
    };

    if !app.tasks.is_empty() {
        app.list_state.select(Some(0));
    }
    run_app(&mut terminal, &mut app)?;
    restore_terminal()?;
    Ok(())
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app(run_app_terminal: &mut Terminal<CrosstermBackend<Stdout>>,run_app_app: &mut App) -> Result<()> {
    loop {
        run_app_terminal.draw(|f| user_interface(f, run_app_app))?;
        if let Event::Key(key) = event::read()? {

            //Filter for Press (ignore Release)
            if key.kind == KeyEventKind::Press {
                //NOTE TO SELF:
                // -> deals with type (what we make)
                // => deals with logical (what should we do)
                if key.code == KeyCode::Char('q') {
                    run_app_app.export()?;
                        run_app_app.should_quit = true;
                        return Ok(());
                    }
                    match run_app_app.focus {
                         Focus::TaskList => {
                             match key.code {
                                 KeyCode::Down => run_app_app.next_task(),
                                 KeyCode::Up => run_app_app.previous_task(),
                                 KeyCode::Right => run_app_app.focus = Focus::CalendarList,
                                 KeyCode::Enter => run_app_app.focus = Focus::TaskDetail,
                                 KeyCode::Esc => run_app_app.focus = Focus::TaskList,
                                 KeyCode::Backspace => run_app_app.delete_task(),
                                 _=> {} //ignoring ANY other keys
                             }
                         }
                        Focus::CalendarList => {
                            if key.code == KeyCode::Char('t'){
                                run_app_app.focus = Focus::TaskList;
                            }
                            match key.code {
                                KeyCode::Enter => { run_app_app.focus = Focus::NewTaskCreation; run_app_app.input_buffer.clear(); run_app_app.input_priority = 1;}
                                KeyCode::Right => run_app_app.next_day(),
                                KeyCode::Down => run_app_app.next_week(),
                                KeyCode::Left => run_app_app.previous_day(),
                                KeyCode::Up => run_app_app.previous_week(),
                                _=> {}
                            }
                        }
                        Focus::TaskDetail =>{
                            if let KeyCode::Esc = key.code {
                                run_app_app.focus = Focus::TaskList;
                            }
                        }
                        Focus::NewTaskCreation => {
                            match key.code {
                                KeyCode::Char(c) => { run_app_app.input_buffer.push(c); }
                                KeyCode::Backspace => { run_app_app.input_buffer.pop(); }
                                KeyCode::Esc => { run_app_app.focus = Focus::CalendarList; }

                                KeyCode::Enter => {
                                    if !run_app_app.input_buffer.is_empty() {
                                        let clean_input = run_app_app.input_buffer.trim();
                                        let (final_name, final_priority) = match run_app_app.input_buffer.rsplit_once(' ') {
                                            Some((text_part, number_part)) => {
                                                //we found a space! Trying to turn the right part into a number
                                                match number_part.parse::<u8>() {
                                                    Ok(p) => (text_part.to_string(), p),
                                                    Err(_) => (run_app_app.input_buffer.clone(), 1), //
                                                }
                                            },
                                            None => (clean_input.to_string(), 1), // No space at all
                                        };

                                        let deadline_date = run_app_app.current_date.with_day(run_app_app.calendar_curosr_day).unwrap();

                                        let new_task = Task {
                                            event_name: final_name,
                                            completed: false,
                                            deadline: Some(deadline_date),
                                            priority: final_priority,
                                        };
                                        run_app_app.tasks.push(new_task);

                                        run_app_app.input_buffer.clear();
                                        run_app_app.focus = Focus::CalendarList;
                                    }
                                }
                                _=> {}
                            }
                        }

                    }
            }
        }
     }
}

fn user_interface(ui_frame: &mut Frame, ui_app: &mut App) {
    let layout_split = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(30), Constraint::Percentage(70)]).split(ui_frame.area());

        let created_tasks: Vec<ListItem> = ui_app.tasks.iter().map(|task| {
            ListItem::new(task.event_name.as_str())
        }).collect();

        let (list_style, list_symbol) = match ui_app.focus {
            Focus::TaskList | Focus::TaskDetail => ( // | Focus::task_detail is the syntax we use to use same logic for both
                                                     Style::default().add_modifier(Modifier::REVERSED), ">>"),
            _=> (Style::default(), " "),
        };

        let task_list = List::new(created_tasks)
            .block(Block::default().borders(Borders::ALL).title("Tasks"))
            .highlight_style(list_style)
            .highlight_symbol(list_symbol);


        ui_frame.render_stateful_widget(task_list, layout_split[0], &mut ui_app.list_state);

        if let Focus::TaskDetail = ui_app.focus {
            if let Some(chosen_task) = ui_app.list_state.selected() {
                let chosen_task = &ui_app.tasks[chosen_task];

                let deadline_text = match chosen_task.deadline {
                    Some(date) =>date.format("%d-%B-%Y").to_string(),
                    None => "No Deadline".to_string(),
                };

                let priority_style = match chosen_task.priority {
                    5 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD), // Urgent!
                    4 => Style::default().fg(Color::LightRed),
                    3 => Style::default().fg(Color::Yellow),
                    2 => Style::default().fg(Color::LightGreen),
                    _ => Style::default().fg(Color::Green), // 1 or 0 (Low priority)
                };

                let details_block = Paragraph::new(format!(
                    "Task: {}\n\nPriority: {}\n\nDue date: {}",
                    chosen_task.event_name, chosen_task.priority,deadline_text
                )).block(Block::default().borders(Borders::ALL).title("Task details")).style(priority_style);

                ui_frame.render_widget(details_block, layout_split[1]);
            }
        } else if let Focus::NewTaskCreation = ui_app.focus{
            let lines = vec![Line::from("Create New Task"), Line::from(""), Line::from("Enter Task Name: "), Line::from(vec![Span::raw(" > "), Span::styled(ui_app.input_buffer.clone(), Style::default().fg(Color::LightYellow))
            ]),
            Line::from(""),Line::from("Press 'Enter' to save, 'Esc' to cancel")];
            let create_block = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));

            ui_frame.render_widget(create_block, layout_split[1]);
        }else{
            let calendar_area = layout_split[1];

            let calendar_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), Constraint::Length(1), Constraint::Min(0)
                ]).split(calendar_area);

            let header_area = calendar_chunks[0];
            let day_names_area = calendar_chunks[1];
            let date_grid_area = calendar_chunks[2];

            let header = Paragraph::new(ui_app.current_date.format("%B %Y").to_string())
                .alignment(Alignment::Center);
            ui_frame.render_widget(header, header_area);

            let day_name_cells = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(1, 7); 7])
                .split(day_names_area);

            let day_names = ["Mon", "Tue", "Wed", "Thu", "Fr", "Sat", "Sun"];
            for (i, name) in day_names.iter().enumerate() {
                let cell = Paragraph::new(*name).alignment(Alignment::Center);
                ui_frame.render_widget(cell, day_name_cells[i]);
            }

            let week_rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 6); 6])
                .split(date_grid_area);

            let mut day_counter = 1;
            let first_day = ui_app.current_date.with_day(1).unwrap();
            let starting_day_offset = (first_day.weekday().number_from_monday() - 1) as usize;
            let days_in_month = ui_app.days_in_current_month();

            for (row_index, week_row) in week_rows.iter().enumerate() {
                let day_cells = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Ratio(1, 7); 7])
                    .split(*week_row);


                for (col_index, day_cell) in day_cells.iter().enumerate() {
                    let cell_index = row_index * 7 + col_index;
                    if cell_index >= starting_day_offset && day_counter <= days_in_month {
                        let current_style = if day_counter == ui_app.calendar_curosr_day { //today I learned you can use if when assigning the value :)))
                            match ui_app.focus {
                                Focus::CalendarList => Style::default().add_modifier(Modifier::REVERSED),
                                _ => Style::default(),
                            }
                        } else {
                            Style::default()
                        };

                        let day_num = Paragraph::new(day_counter.to_string()).alignment(Alignment::Center)
                            .block(Block::default().borders(Borders::ALL)).style(current_style);
                        ui_frame.render_widget(day_num,*day_cell);
                        day_counter += 1;
                    }
                }
            }
        }
    }
