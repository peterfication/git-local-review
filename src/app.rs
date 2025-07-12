use crate::database::Database;
use crate::event::{AppEvent, Event, EventHandler};
use crate::models::review::Review;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

/// Application.
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Event handler.
    pub events: EventHandler,
    /// Database connection.
    pub database: Database,
    /// Reviews list.
    pub reviews: Vec<Review>,
    /// Show create review popup.
    pub review_create_popup_show: bool,
    /// Title input for new review.
    pub review_create_title_input: String,
}

impl Default for App {
    fn default() -> Self {
        panic!("Use App::new() instead of Default");
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub async fn new() -> color_eyre::Result<Self> {
        let database = Database::new().await?;
        let reviews = Review::list_all(database.pool()).await.unwrap_or_default();

        Ok(Self {
            running: true,
            events: EventHandler::new(),
            database,
            reviews,
            review_create_popup_show: false,
            review_create_title_input: String::new(),
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                #[allow(clippy::single_match)]
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event) => self.handle_key_events(key_event)?,
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                    AppEvent::ReviewCreateOpen => self.review_create_open(),
                    AppEvent::ReviewCreateClose => self.review_create_close(),
                    AppEvent::ReviewCreateSubmit => self.review_create_submit().await?,
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        if self.review_create_popup_show {
            self.handle_popup_keys(key_event)?;
        } else {
            match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.events.send(AppEvent::Quit)
                }
                KeyCode::Char('n') => self.events.send(AppEvent::ReviewCreateOpen),
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_popup_keys(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => self.events.send(AppEvent::ReviewCreateClose),
            KeyCode::Enter => self.events.send(AppEvent::ReviewCreateSubmit),
            KeyCode::Char(char) => {
                self.review_create_title_input.push(char);
            }
            KeyCode::Backspace => {
                self.review_create_title_input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn review_create_open(&mut self) {
        self.review_create_popup_show = true;
        self.review_create_title_input.clear();
    }

    pub fn review_create_close(&mut self) {
        self.review_create_popup_show = false;
        self.review_create_title_input.clear();
    }

    pub async fn review_create_submit(&mut self) -> color_eyre::Result<()> {
        if !self.review_create_title_input.trim().is_empty() {
            let review = Review::new(self.review_create_title_input.trim().to_string());
            review.save(self.database.pool()).await?;
            self.reviews = Review::list_all(self.database.pool())
                .await
                .unwrap_or_default();
            log::info!("Created review: {}", review.title);
        }
        self.review_create_close();
        Ok(())
    }
}
