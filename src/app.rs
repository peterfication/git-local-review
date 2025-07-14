use crate::database::Database;
use crate::event::{EventHandler, ReviewCreateData};
use crate::event_handler::EventProcessor;
use crate::models::review::Review;
use crate::services::ReviewService;
use crate::views::{ViewHandler, main::MainView, review_create::ReviewCreateView};
use ratatui::{DefaultTerminal, crossterm::event::KeyEvent};

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
    pub view_stack: Vec<Box<dyn ViewHandler>>,
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
        let reviews = ReviewService::list_reviews(&database).await?;

        Ok(Self {
            running: true,
            events: EventHandler::new(),
            database,
            reviews,
            view_stack: vec![Box::new(MainView)],
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
    /// Only the top view in the stack will handle the key events.
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        // We need to avoid borrowing self twice, so we'll extract the view temporarily
        if !self.view_stack.is_empty() {
            let mut current_view = self.view_stack.pop().unwrap();
            let result = current_view.handle_key_events(self, key_event);
            self.view_stack.push(current_view);
            result?;
        }
        Ok(())
    }

    /// Push a view onto the view stack.
    pub fn push_view(&mut self, view: Box<dyn ViewHandler>) {
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
        self.push_view(Box::new(ReviewCreateView::default()));
    }

    pub fn review_create_close(&mut self) {
        self.pop_view();
    }

    pub async fn review_create_submit(&mut self, data: ReviewCreateData) -> color_eyre::Result<()> {
        self.reviews = ReviewService::create_review(&self.database, data).await?;
        self.review_create_close();
        Ok(())
    }
}
