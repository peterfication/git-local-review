use std::{sync::Arc, time::Duration};

use color_eyre::eyre::OptionExt;
use futures::{FutureExt, StreamExt};
use ratatui::crossterm::event::{Event as CrosstermEvent, KeyEvent};
use tokio::sync::mpsc;

use crate::{
    models::{Comment, Review},
    services::{
        CommentsLoadParams, CommentsLoadingState, GitBranchesLoadingState, GitDiffLoadingState,
        ReviewCreateData, ReviewLoadingState, ReviewsLoadingState,
    },
    views::KeyBinding,
};

/// Type alias for review identifiers to make event signatures more descriptive.
pub type ReviewId = str;

/// The frequency at which tick events are emitted.
const TICK_FPS: f64 = 30.0;

/// Representation of all possible events.
#[derive(Clone, Debug)]
pub enum Event {
    /// An event that is emitted on a regular schedule.
    ///
    /// Use this event to run any code which has to run outside of being a direct response to a user
    /// event. e.g. polling exernal systems, updating animations, or rendering the UI based on a
    /// fixed frame rate.
    Tick,
    /// Crossterm events.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Application events.
    ///
    /// Use this event to emit custom events that are specific to your application.
    App(AppEvent),
}

/// Application events.
///
/// You can extend this enum with your own custom events.
#[derive(Clone, Debug)]
pub enum AppEvent {
    /// Initialization event.
    Init,
    /// Check all reviews against current Git repository state.
    ReviewsBranchStatusCheck,
    /// Quit the application.
    Quit,
    /// Close the current view.
    ViewClose,

    /// Trigger loading of reviews.
    ReviewsLoad,
    /// Load the reviews from the database
    ReviewsLoading,
    /// Propagates the current loading state of reviews.
    ReviewsLoadingState(ReviewsLoadingState),
    /// Load a single review by ID.
    ReviewLoad(Arc<ReviewId>),
    /// Propagates the current loading state of a single review.
    ReviewLoadingState(ReviewLoadingState),
    /// Single review loaded successfully.
    ReviewLoaded(Arc<Review>),
    /// Review was not found.
    ReviewNotFound(Arc<ReviewId>),
    /// Error occurred while loading a review.
    ReviewLoadError(Arc<ReviewId>),
    /// Inform that a review has been created.
    ReviewCreated(Review),
    /// Error occurred while creating a review.
    ReviewCreatedError(Arc<str>),
    /// Delete the selected review.
    ReviewDelete(Arc<ReviewId>),
    /// Inform that a review has been deleted.
    ReviewDeleted,
    /// Error occurred while deleting a review.
    ReviewDeletedError(Arc<ReviewId>),

    /// Open help modal with keybindings.
    HelpOpen(Arc<[KeyBinding]>),
    /// Key selected from help modal.
    HelpKeySelected(Arc<KeyEvent>),

    /// Open the review creation view.
    ReviewCreateOpen,
    /// Submit the review creation form.
    ReviewCreateSubmit(Arc<ReviewCreateData>),

    /// Open delete confirmation dialog for selected review.
    ReviewDeleteConfirm(Arc<ReviewId>),

    /// Open review details view.
    ReviewDetailsOpen(Arc<ReviewId>),

    /// Trigger loading of Git branches.
    GitBranchesLoad,
    /// Load the Git branches
    GitBranchesLoading,
    /// Propagates the current loading state of Git branches.
    GitBranchesLoadingState(GitBranchesLoadingState),

    /// Trigger loading of Git diff between two SHAs.
    GitDiffLoad {
        base_sha: Arc<str>,
        target_sha: Arc<str>,
    },
    /// Load the Git diff
    GitDiffLoading {
        base_sha: Arc<str>,
        target_sha: Arc<str>,
    },
    /// Propagates the current loading state of Git diff.
    GitDiffLoadingState(GitDiffLoadingState),

    /// Toggle the view status of a file for a review.
    FileViewToggle {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
    },
    /// File view status was toggled successfully.
    FileViewToggled {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        is_viewed: bool,
    },
    /// Error occurred while toggling file view status.
    FileViewToggleError {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        error: Arc<str>,
    },
    /// Load the viewed files for a review.
    FileViewsLoad { review_id: Arc<ReviewId> },
    /// File views are being loaded.
    FileViewsLoading { review_id: Arc<ReviewId> },
    /// File views were loaded successfully.
    FileViewsLoaded {
        review_id: Arc<ReviewId>,
        viewed_files: Arc<Vec<String>>,
    },
    /// Error occurred while loading file views.
    FileViewsLoadError {
        review_id: Arc<ReviewId>,
        error: Arc<str>,
    },

    /// Open comments view for a file or line.
    CommentsOpen {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
    },
    /// Load comments for a review, file or line.
    CommentsLoad(CommentsLoadParams),
    /// Comments are being loaded.
    CommentsLoading(CommentsLoadParams),
    /// Propagates the current loading state of comments.
    CommentsLoadingState {
        params: CommentsLoadParams,
        state: CommentsLoadingState,
    },
    /// Create a new comment.
    CommentCreate {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
        content: Arc<str>,
    },
    /// Comment was created successfully.
    CommentCreated(Arc<Comment>),
    /// Error occurred while creating a comment.
    CommentCreateError(Arc<str>),
    /// Mark a comment as resolved.
    CommentMarkResolved { comment_id: Arc<str> },
    /// Comment was marked as resolved successfully.
    CommentMarkedResolved { comment_id: Arc<str> },
    /// Error occurred while marking comment as resolved.
    CommentMarkResolvedError {
        comment_id: Arc<str>,
        error: Arc<str>,
    },
    /// Toggle a comment's resolved state.
    CommentToggleResolved { comment_id: Arc<str> },
    /// Comment's resolved state was toggled successfully.
    CommentToggledResolved {
        comment_id: Arc<str>,
        resolved: bool,
    },
    /// Error occurred while toggling comment resolved state.
    CommentToggleResolvedError {
        comment_id: Arc<str>,
        error: Arc<str>,
    },
    /// Mark all comments as resolved for a specific target.
    CommentsMarkAllResolved {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
    },
    /// All comments were marked as resolved successfully.
    CommentsMarkedAllResolved {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
    },
    /// Error occurred while marking all comments as resolved.
    CommentsMarkAllResolvedError {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
        error: Arc<str>,
    },
    /// Toggle resolved state for all comments in a specific target.
    CommentsToggleAllResolved {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
    },
    /// All comments resolved state was toggled successfully.
    CommentsToggledAllResolved {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
        resolved_count: usize,
        unresolved_count: usize,
    },
    /// Error occurred while toggling all comments resolved state.
    CommentsToggleAllResolvedError {
        review_id: Arc<ReviewId>,
        file_path: Arc<str>,
        line_number: Option<i64>,
        error: Arc<str>,
    },
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Arc<Event>>,
    /// Event receiver channel.
    receiver: mpsc::UnboundedReceiver<Arc<Event>>,
}

impl Default for EventHandler {
    fn default() -> Self {
        panic!("Use EventHandler::new() instead of Default");
    }
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`] and spawns a new thread to handle events.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = EventTask::new(sender.clone());
        tokio::spawn(async { actor.run().await });
        Self { sender, receiver }
    }

    /// Constructs a new instance of [`EventHandler`] for testing without spawning the event task.
    /// This allows tests to control event flow manually.
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }

    /// Receives an event from the sender.
    ///
    /// This function blocks until an event is received.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sender channel is disconnected. This can happen if an
    /// error occurs in the event thread. In practice, this should not happen unless there is a
    /// problem with the underlying terminal.
    pub async fn next(&mut self) -> color_eyre::Result<Arc<Event>> {
        self.receiver
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }

    /// Queue an app event to be sent to the event receiver.
    ///
    /// This is useful for sending events to the event handler which will be processed by the next
    /// iteration of the application's event loop.
    pub fn send(&mut self, app_event: AppEvent) {
        // Ignore the result as the reciever cannot be dropped while this struct still has a
        // reference to it
        let _ = self.sender.send(Event::App(app_event).into());
    }

    /// Queue a key event to be sent to the event receiver as a crossterm event.
    ///
    /// This is useful for programmatically sending key events that will be processed
    /// through the normal key event handling flow.
    pub fn send_key_event(&mut self, key_event: KeyEvent) {
        let crossterm_event = CrosstermEvent::Key(key_event);
        let _ = self.sender.send(Event::Crossterm(crossterm_event).into());
    }

    /// Check if there are any pending events in the queue.
    /// This is useful for testing to verify that events have been sent.
    #[cfg(test)]
    pub fn has_pending_events(&self) -> bool {
        !self.receiver.is_empty()
    }

    /// Try to receive an event without blocking.
    /// Returns None if no events are available.
    /// This is useful for testing to check what events have been sent.
    #[cfg(test)]
    pub fn try_recv(&mut self) -> Option<Arc<Event>> {
        self.receiver.try_recv().ok()
    }
}

/// A thread that handles reading crossterm events and emitting tick events on a regular schedule.
struct EventTask {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Arc<Event>>,
}

impl EventTask {
    /// Constructs a new instance of [`Event`].
    fn new(sender: mpsc::UnboundedSender<Arc<Event>>) -> Self {
        Self { sender }
    }

    /// Runs the event thread.
    ///
    /// This function emits tick events at a fixed rate and polls for crossterm events in between.
    async fn run(self) -> color_eyre::Result<()> {
        let tick_rate = Duration::from_secs_f64(1.0 / TICK_FPS);
        let mut reader = crossterm::event::EventStream::new();
        let mut tick = tokio::time::interval(tick_rate);
        loop {
            let tick_delay = tick.tick();
            let crossterm_event = reader.next().fuse();
            tokio::select! {
              _ = self.sender.closed() => {
                break;
              }
              _ = tick_delay => {
                self.send(Event::Tick.into());
              }
              Some(Ok(event)) = crossterm_event => {
                self.send(Event::Crossterm(event).into());
              }
            };
        }
        Ok(())
    }

    /// Sends an event to the receiver.
    fn send(&self, event: Arc<Event>) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ratatui::crossterm::event::{KeyCode, KeyEventKind, KeyEventState, KeyModifiers};

    #[test]
    fn test_send_key_event() {
        let mut event_handler = EventHandler::new_for_test();

        let key_event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        // Send a key event
        event_handler.send_key_event(key_event);

        // Verify the event was sent as a crossterm event
        assert!(event_handler.has_pending_events());
        let event = event_handler.try_recv().unwrap();

        match *event {
            Event::Crossterm(CrosstermEvent::Key(received_key_event)) => {
                assert_eq!(received_key_event.code, KeyCode::Char('q'));
                assert_eq!(received_key_event.modifiers, KeyModifiers::empty());
                assert_eq!(received_key_event.kind, KeyEventKind::Press);
                assert_eq!(received_key_event.state, KeyEventState::empty());
            }
            _ => panic!("Expected crossterm key event, got: {event:?}"),
        }
    }
}
