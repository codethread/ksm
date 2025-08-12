use anyhow::Result;
use log::{debug, warn};
use std::cell::RefCell;
use std::env;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitStatus, Output};

pub trait CommandExecutor {
    fn execute_ls_command(&self, command: KittenLsCommand) -> Result<Output>;
    fn execute_focus_tab_command(
        &self,
        command: KittenFocusTabCommand,
    ) -> Result<std::process::ExitStatus>;
    fn execute_launch_command(
        &self,
        command: KittenLaunchCommand,
    ) -> Result<std::process::ExitStatus>;
}

pub struct KittyExecutor {
    socket: String,
}

impl KittyExecutor {
    pub fn new() -> Self {
        let socket = get_kitty_socket();
        Self { socket }
    }
}

impl Default for KittyExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandExecutor for KittyExecutor {
    fn execute_ls_command(&self, command: KittenLsCommand) -> Result<Output> {
        let socket_arg = format!("--to={}", self.socket);
        let mut args = vec!["@", &socket_arg, "ls"];

        let match_formatted;
        if let Some(match_arg) = &command.match_arg {
            match_formatted = format!("--match={}", match_arg);
            args.push(&match_formatted);
            debug!(
                "Running kitten @ --to={} ls --match={}",
                self.socket, match_arg
            );
        } else {
            debug!("Running kitten @ --to={} ls", self.socket);
        }

        Ok(Command::new("kitten").args(&args).output()?)
    }

    fn execute_focus_tab_command(
        &self,
        command: KittenFocusTabCommand,
    ) -> Result<std::process::ExitStatus> {
        let socket_arg = format!("--to={}", self.socket);
        let match_arg = format!("--match=id:{}", command.tab_id);
        let args = ["@", &socket_arg, "focus-tab", &match_arg];

        debug!(
            "Running kitten @ --to={} focus-tab --match=id:{}",
            self.socket, command.tab_id
        );

        Ok(Command::new("kitten").args(args).status()?)
    }

    fn execute_launch_command(
        &self,
        command: KittenLaunchCommand,
    ) -> Result<std::process::ExitStatus> {
        let socket_arg = format!("--to={}", self.socket);
        let type_arg = format!("--type={}", command.launch_type);
        let mut args = vec!["@", &socket_arg, "launch", &type_arg];

        let cwd_formatted;
        if let Some(cwd) = &command.cwd {
            cwd_formatted = format!("--cwd={}", cwd);
            args.push(&cwd_formatted);
        }

        let env_formatted;
        if let Some(env) = &command.env {
            env_formatted = format!("--env={}", env);
            args.push(&env_formatted);
        }

        if let Some(tab_title) = &command.tab_title {
            args.push("--tab-title");
            args.push(tab_title);
        }

        debug!(
            "Running kitten @ --to={} launch --type={} {}{}{}",
            self.socket,
            command.launch_type,
            command
                .cwd
                .as_ref()
                .map(|c| format!("--cwd={} ", c))
                .unwrap_or_default(),
            command
                .env
                .as_ref()
                .map(|e| format!("--env={} ", e))
                .unwrap_or_default(),
            command
                .tab_title
                .as_ref()
                .map(|t| format!("--tab-title={}", t))
                .unwrap_or_default()
        );

        Ok(Command::new("kitten").args(&args).status()?)
    }
}

#[derive(Debug, Clone)]
pub struct KittenLsCommand {
    pub match_arg: Option<String>,
}

impl Default for KittenLsCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl KittenLsCommand {
    pub fn new() -> Self {
        Self { match_arg: None }
    }

    pub fn match_env(mut self, env_var: &str, value: &str) -> Self {
        self.match_arg = Some(format!("env:{}={}", env_var, value));
        self
    }
}

#[derive(Debug, Clone)]
pub struct KittenFocusTabCommand {
    pub tab_id: u32,
}

impl KittenFocusTabCommand {
    pub fn new(tab_id: u32) -> Self {
        Self { tab_id }
    }
}

#[derive(Debug, Clone)]
pub struct KittenLaunchCommand {
    pub launch_type: String,
    pub cwd: Option<String>,
    pub env: Option<String>,
    pub tab_title: Option<String>,
}

impl Default for KittenLaunchCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl KittenLaunchCommand {
    pub fn new() -> Self {
        Self {
            launch_type: "tab".to_string(),
            cwd: None,
            env: None,
            tab_title: None,
        }
    }

    pub fn launch_type(mut self, launch_type: &str) -> Self {
        self.launch_type = launch_type.to_string();
        self
    }

    pub fn cwd(mut self, cwd: &str) -> Self {
        self.cwd = Some(cwd.to_string());
        self
    }

    pub fn env(mut self, env_var: &str, value: &str) -> Self {
        self.env = Some(format!("{}={}", env_var, value));
        self
    }

    pub fn tab_title(mut self, title: &str) -> Self {
        self.tab_title = Some(title.to_string());
        self
    }
}

#[derive(Debug)]
pub struct MockExecutor {
    pub ls_calls: RefCell<Vec<KittenLsCommand>>,
    pub focus_tab_calls: RefCell<Vec<KittenFocusTabCommand>>,
    pub launch_calls: RefCell<Vec<KittenLaunchCommand>>,
    pub ls_responses: RefCell<Vec<Result<Output>>>,
    pub focus_tab_responses: RefCell<Vec<Result<ExitStatus>>>,
    pub launch_responses: RefCell<Vec<Result<ExitStatus>>>,
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

    pub fn expect_ls_response(&self, response: Result<Output>) {
        self.ls_responses.borrow_mut().push(response);
    }

    pub fn expect_focus_tab_response(&self, response: Result<ExitStatus>) {
        self.focus_tab_responses.borrow_mut().push(response);
    }

    pub fn expect_launch_response(&self, response: Result<ExitStatus>) {
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
    fn execute_ls_command(&self, command: KittenLsCommand) -> Result<Output> {
        self.ls_calls.borrow_mut().push(command);
        self.ls_responses.borrow_mut().pop().unwrap_or_else(|| {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: b"[]".to_vec(),
                stderr: Vec::new(),
            })
        })
    }

    fn execute_focus_tab_command(&self, command: KittenFocusTabCommand) -> Result<ExitStatus> {
        self.focus_tab_calls.borrow_mut().push(command);
        self.focus_tab_responses
            .borrow_mut()
            .pop()
            .unwrap_or_else(|| Ok(ExitStatus::from_raw(0)))
    }

    fn execute_launch_command(&self, command: KittenLaunchCommand) -> Result<ExitStatus> {
        self.launch_calls.borrow_mut().push(command);
        self.launch_responses
            .borrow_mut()
            .pop()
            .unwrap_or_else(|| Ok(ExitStatus::from_raw(0)))
    }
}

impl Default for MockExecutor {
    fn default() -> Self {
        Self::with_default_socket()
    }
}

fn get_kitty_socket() -> String {
    if let Ok(socket) = env::var("KITTY_LISTEN_ON") {
        debug!("Using KITTY_LISTEN_ON environment variable: {socket}");
        return socket;
    }

    debug!("KITTY_LISTEN_ON not set, searching for socket files");

    // Find socket file
    if let Ok(output) = Command::new("sh")
        .arg("-c")
        .arg("ls /tmp/mykitty* 2>/dev/null | head -1")
        .output()
    {
        if let Ok(socket_file) = String::from_utf8(output.stdout) {
            let socket_file = socket_file.trim();
            if !socket_file.is_empty() {
                let socket_path = format!("unix:{}", socket_file);
                debug!("Found socket file: {}", socket_path);
                return socket_path;
            }
        }
    }

    let default_socket = "unix:/tmp/mykitty".to_string();
    warn!("No socket file found, using default: {}", default_socket);
    default_socket
}
