use crate::database::Database;
use crate::event::{AppEvent, EventHandler, ReviewCreateData};
use crate::event_handler::EventProcessor;
use crate::models::review::Review;
use crate::views::{View, main::MainView, review_create::ReviewCreateView};
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
    /// Current view stack.
    pub view_stack: Vec<View>,
    /// Main view instance.
    pub main_view: MainView,
    /// Review create view instance.
    pub review_create_view: ReviewCreateView,
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
            view_stack: vec![View::Main],
            main_view: MainView,
            review_create_view: ReviewCreateView::default(),
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            let event = self.events.next().await?;
            EventProcessor::process_event(&mut self, event).await?;
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        let current_view = self.current_view();
        match current_view {
            View::Main => self.handle_main_key_events(key_event)?,
            View::ReviewCreate => self.handle_review_create_key_events(key_event)?,
        }
        Ok(())
    }

    fn handle_main_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Char('n') => self.events.send(AppEvent::ReviewCreateOpen),
            _ => {}
        }
        Ok(())
    }

    fn handle_review_create_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc => {
                self.review_create_view.title_input.clear();
                self.events.send(AppEvent::ReviewCreateClose);
            }
            KeyCode::Enter => {
                self.events
                    .send(AppEvent::ReviewCreateSubmit(ReviewCreateData {
                        title: self.review_create_view.title_input.clone(),
                    }));
                self.review_create_view.title_input.clear();
            }
            KeyCode::Char(char) => {
                self.review_create_view.title_input.push(char);
            }
            KeyCode::Backspace => {
                self.review_create_view.title_input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    /// Get the current view from the view stack.
    pub fn current_view(&self) -> View {
        self.view_stack.last().cloned().unwrap_or_default()
    }

    /// Push a view onto the view stack.
    pub fn push_view(&mut self, view: View) {
        self.view_stack.push(view);
    }

    /// Pop the current view from the view stack.
    pub fn pop_view(&mut self) {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
        }
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
        self.push_view(View::ReviewCreate);
    }

    pub fn review_create_close(&mut self) {
        self.pop_view();
    }

    pub async fn review_create_submit(&mut self, data: ReviewCreateData) -> color_eyre::Result<()> {
        if !data.title.trim().is_empty() {
            let review = Review::new(data.title.trim().to_string());
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
