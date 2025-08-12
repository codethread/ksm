use anyhow::Result;
use std::cell::RefCell;

use crate::commands::focus_tab::KittenFocusTabCommand;
use crate::commands::launch::KittenLaunchCommand;
use crate::commands::ls::KittenLsCommand;
use crate::executor::CommandExecutor;
use crate::types::{KittyCommandResult, KittyLaunchResponse, KittyLsResponse};

#[derive(Debug)]
pub struct MockExecutor {
    pub ls_calls: RefCell<Vec<KittenLsCommand>>,
    pub focus_tab_calls: RefCell<Vec<KittenFocusTabCommand>>,
    pub launch_calls: RefCell<Vec<KittenLaunchCommand>>,
    pub ls_responses: RefCell<Vec<Result<KittyLsResponse>>>,
    pub focus_tab_responses: RefCell<Vec<Result<KittyCommandResult<()>>>>,
    pub launch_responses: RefCell<Vec<Result<KittyCommandResult<KittyLaunchResponse>>>>,
}

impl MockExecutor {
    pub fn new() -> Self {
        Self {
            ls_calls: RefCell::new(Vec::new()),
            focus_tab_calls: RefCell::new(Vec::new()),
            launch_calls: RefCell::new(Vec::new()),
            ls_responses: RefCell::new(Vec::new()),
            focus_tab_responses: RefCell::new(Vec::new()),
            launch_responses: RefCell::new(Vec::new()),
        }
    }

    pub fn with_default_socket() -> Self {
        Self::new()
    }

    pub fn expect_ls_response(&self, response: Result<KittyLsResponse>) {
        self.ls_responses.borrow_mut().push(response);
    }

    pub fn expect_focus_tab_response(&self, response: Result<KittyCommandResult<()>>) {
        self.focus_tab_responses.borrow_mut().push(response);
    }

    pub fn expect_launch_response(
        &self,
        response: Result<KittyCommandResult<KittyLaunchResponse>>,
    ) {
        self.launch_responses.borrow_mut().push(response);
    }

    pub fn ls_call_count(&self) -> usize {
        self.ls_calls.borrow().len()
    }

    pub fn focus_tab_call_count(&self) -> usize {
        self.focus_tab_calls.borrow().len()
    }

    pub fn launch_call_count(&self) -> usize {
        self.launch_calls.borrow().len()
    }

    pub fn get_ls_calls(&self) -> Vec<KittenLsCommand> {
        self.ls_calls.borrow().clone()
    }

    pub fn get_focus_tab_calls(&self) -> Vec<KittenFocusTabCommand> {
        self.focus_tab_calls.borrow().clone()
    }

    pub fn get_launch_calls(&self) -> Vec<KittenLaunchCommand> {
        self.launch_calls.borrow().clone()
    }
}

impl CommandExecutor for &MockExecutor {
    fn ls(&self, command: KittenLsCommand) -> Result<KittyLsResponse> {
        self.ls_calls.borrow_mut().push(command);
        let response = self.ls_responses.borrow_mut().pop().unwrap_or_else(|| {
            Ok(Vec::new()) // Return empty list by default
        });
        response
    }

    fn focus_tab(&self, command: KittenFocusTabCommand) -> Result<KittyCommandResult<()>> {
        self.focus_tab_calls.borrow_mut().push(command);
        let response = self
            .focus_tab_responses
            .borrow_mut()
            .pop()
            .unwrap_or_else(|| Ok(KittyCommandResult::success_empty()));
        response
    }

    fn launch(
        &self,
        command: KittenLaunchCommand,
    ) -> Result<KittyCommandResult<KittyLaunchResponse>> {
        self.launch_calls.borrow_mut().push(command);
        let response = self.launch_responses.borrow_mut().pop().unwrap_or_else(|| {
            Ok(KittyCommandResult::success(KittyLaunchResponse {
                tab_id: None,
                window_id: None,
            }))
        });
        response
    }
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::with_default_socket()
    }
}
