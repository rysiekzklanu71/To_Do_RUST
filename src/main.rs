use std::io::{self, stdout, Stdout};
use std::ptr::null;
use std::thread::current;
use anyhow::Result;
use chrono::{DateTime, Local, Datelike, format, NaiveDate};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    style::{Color, Modifier, Style},
    layout::{Alignment, Constraint, Direction, Layout, Rect },
    widgets::{Block, Borders, List, ListItem, ListState,Paragraph },
    Frame,
    Terminal,
};



struct Task{
    event_name: String,
    completed: bool,
    deadline: Option<DateTime<Local>>,
    priority: u8,
}

enum Focus{
    task_list,
    calendar_list,
}
struct App{
    tasks: Vec<Task>,
    should_quit: bool,
    list_state: ListState,
    current_date: DateTime<Local>,
    focus: Focus,
    calendar_curosr_day: u32 //u32 for chrono use cuz it uses u32 and casting u8 non stop would be a burden not worth saving a few bytes
}

impl App {
    fn next(&mut self){
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


    fn previous(&mut self){
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

    fn calendar_list_picked(&mut self){

    }
}


fn main() -> Result<()> {
    let mut terminal = init_terminal()?;
    let mut app = App{ should_quit: false, list_state: ListState::default(), current_date: Local::now(), focus: Focus::task_list, calendar_curosr_day: Local::now().day(), //test V1
        tasks: vec![
        Task{
            event_name: "Algorithms Exam".to_string(),
            completed: false,
            deadline: None,
            priority: 5,
        },
        Task{
            event_name: "Rent payment".to_string(),
            completed: true,
            deadline: None,
            priority: 2,
        }
    ],
    };
    let result = run_app(&mut terminal, &mut app);

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

fn run_app(run_app_terminal: &mut Terminal<CrosstermBackend<Stdout>>,run_app_App: &mut App) -> Result<()> {
    loop {
        run_app_terminal.draw(|f| user_interface(f, run_app_App))?;
        if let Event::Key(key) = event::read()? {

            // 2 Filter for Press (ignore Release)
            if key.kind == KeyEventKind::Press {
                //NOTE TO SELF:
                // -> deals with type (what we make)
                // => deals with logical (what should we do)
                match key.code {
                    KeyCode::Char('q') => {
                        run_app_App.should_quit = true;
                        return Ok(());
                    }
                    // if focus ==task_list (below the down, up we wrote executes)

                    KeyCode::Down => run_app_App.next(), // Now 'key' is still in scope!
                    KeyCode::Up => run_app_App.previous(),
                    _ => {}
                    //if focus == calendar { (below all arrow keys logic) }
                }//else match run_app_App.focus {

                }
            }
        }
    }
//}
fn user_interface(ui_frame: &mut Frame, ui_app: &mut App) {
    let layout_split = Layout::default()
    .direction(Direction::Horizontal).constraints([Constraint::Percentage(30), Constraint::Percentage(70)]).split(ui_frame.area());

    /*let left_block = Block::default()
        .title("Tasks")
        .borders(Borders::ALL);
    ui_frame.render_widget(left_block, layout_split[0]);
    */

    let created_tasks: Vec<ListItem> = ui_app.tasks.iter().map(|task|{
        ListItem::new(task.event_name.as_str())
    }).collect();

    let task_list = List::new(created_tasks)
        .block(Block::default().borders(Borders::ALL).title("Tasks"))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol(">>");


    ui_frame.render_stateful_widget(task_list, layout_split[0], &mut ui_app.list_state);

    let calendar_area = layout_split[1];

    let calendar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),Constraint::Length(1),Constraint::Min(0)
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

    let day_names = ["Mon","Tue","Wed","Thu","Fr","Sat","Sun"];
    for (i, name) in day_names.iter().enumerate() {
        let cell = Paragraph::new(*name).alignment(Alignment::Center);
        ui_frame.render_widget(cell, day_name_cells[i]);
    }

    let week_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1,6);6])
        .split(date_grid_area);

    let mut day_counter = 1;
    let first_day = ui_app.current_date.with_day(1).unwrap();
    let starting_day_offset = (first_day.weekday().number_from_monday() - 1) as usize;

    let mut next_month = ui_app.current_date.month();
    let mut next_year = ui_app.current_date.year();
    if(ui_app.current_date.month() == 12){
        next_month = 1;
        next_year = next_year+1;
    }else{ next_month = next_month+1; }

    let first_of_the_next_month = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
        .day();

    for (row_index, week_row) in week_rows.iter().enumerate() {
        let day_cells = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1,7);7])
            .split(*week_row);


        for (col_index, day_cell) in day_cells.iter().enumerate() {
            let cell_index = row_index * 7 + col_index;
            if cell_index >= starting_day_offset && day_counter <= first_of_the_next_month {
                let day_num = Paragraph::new(day_counter.to_string()).alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                ui_frame.render_widget(day_num,*day_cell);
                day_counter += 1;
            }
        }
    }

    /* let right_block = Block::default()
        .borders(Borders::ALL);
    ui_frame.render_widget(right_block, layout_split[1]);
*/


}

//EVERYTHING ON ITS PLACE. MOVE ONTO STAGE ONE OF CALENDAR NAVIGATION