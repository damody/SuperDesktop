//! Bounded dispatcher for providers hosted outside the GPUI shell process.

#[cfg(not(windows))]
compile_error!("shell-provider-host is supported only on Windows targets.");

use std::collections::{BTreeMap, BTreeSet};

use platform_win::common::start_search::{
    SearchLimits, default_application_roots, default_file_roots, discover_applications,
    search_files, settings_catalog,
};
use shell_provider_protocol::{
    CURRENT_PROTOCOL, CommandDescriptor, CommandId, CommandRisk, Envelope, Handshake, HostHealth,
    MAX_FRAME_BYTES, MenuContext, MenuEnumeration, MenuInvocation, MenuInvocationResult,
    ProviderCapability, ProviderRequest, ProviderResponse, ResponseBody, SearchBatch,
    SearchProvider, SearchProviderState, SearchQuery, TerminalKind, ValidationError,
    rank_search_results,
};

pub const DEFAULT_MAX_ACTIVE_REQUESTS: usize = 32;

#[derive(Debug)]
pub struct Dispatcher {
    active: BTreeSet<String>,
    max_active: usize,
    capabilities: Vec<ProviderCapability>,
    menu_generation: u64,
    menu_tokens: BTreeMap<String, RegisteredMenuCommand>,
}

#[derive(Clone, Debug)]
struct RegisteredMenuCommand {
    generation: u64,
    selection_fingerprint: String,
    command_id: String,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ACTIVE_REQUESTS)
    }
}

impl Dispatcher {
    pub fn new(max_active: usize) -> Self {
        assert!(max_active > 0, "provider host capacity must be non-zero");
        Self {
            active: BTreeSet::new(),
            max_active,
            capabilities: vec![
                ProviderCapability::ContextMenu,
                ProviderCapability::JumpList,
                ProviderCapability::SearchApplications,
                ProviderCapability::SearchFiles,
                ProviderCapability::SearchSettings,
                ProviderCapability::NotificationArea,
                ProviderCapability::TaskPreview,
                ProviderCapability::VirtualDesktop,
            ],
            menu_generation: 0,
            menu_tokens: BTreeMap::new(),
        }
    }

    pub fn active_requests(&self) -> usize {
        self.active.len()
    }

    pub fn begin(&mut self, request_id: &str) -> Result<(), TerminalKind> {
        if self.active.contains(request_id) {
            return Err(TerminalKind::InvalidRequest);
        }
        if self.active.len() >= self.max_active {
            return Err(TerminalKind::Busy);
        }
        self.active.insert(request_id.to_owned());
        Ok(())
    }

    pub fn finish(&mut self, request_id: &str) -> bool {
        self.active.remove(request_id)
    }

    pub fn dispatch(
        &mut self,
        request: Envelope<ProviderRequest>,
        now_unix_ms: u64,
    ) -> ProviderResponse {
        if let Err(error) = request.validate_at(now_unix_ms) {
            return terminal_for_validation(&request, error);
        }

        if let ProviderRequest::Cancel { target_request_id } = &request.payload {
            let cancelled = self.finish(target_request_id);
            return response(
                &request,
                if cancelled {
                    TerminalKind::Cancelled
                } else {
                    TerminalKind::Unavailable
                },
                ResponseBody::Empty,
            );
        }

        if let Err(terminal) = self.begin(&request.request_id) {
            return response(&request, terminal, ResponseBody::Empty);
        }

        let result = match &request.payload {
            ProviderRequest::Handshake => response(
                &request,
                TerminalKind::Success,
                ResponseBody::Handshake(Handshake {
                    protocol: CURRENT_PROTOCOL,
                    capabilities: self.capabilities.clone(),
                    max_active_requests: self.max_active,
                    max_frame_bytes: MAX_FRAME_BYTES,
                }),
            ),
            ProviderRequest::Health => response(
                &request,
                TerminalKind::Success,
                ResponseBody::Health(HostHealth {
                    healthy: true,
                    active_requests: self.active.len(),
                    capacity: self.max_active,
                }),
            ),
            ProviderRequest::Execute {
                capability,
                arguments,
            } => {
                let supported = self.capabilities.contains(capability);
                response(
                    &request,
                    if supported {
                        TerminalKind::Success
                    } else {
                        TerminalKind::Unavailable
                    },
                    if supported {
                        ResponseBody::Arguments(arguments.clone())
                    } else {
                        ResponseBody::Empty
                    },
                )
            }
            ProviderRequest::ContextMenuEnumerate(context) => {
                let menu = self.enumerate_menu(context);
                response(&request, TerminalKind::Success, ResponseBody::Menu(menu))
            }
            ProviderRequest::ContextMenuInvoke(invocation) => match self.invoke_menu(invocation) {
                Some(result) => response(
                    &request,
                    TerminalKind::Success,
                    ResponseBody::MenuInvocation(result),
                ),
                None => response(&request, TerminalKind::InvalidRequest, ResponseBody::Empty),
            },
            ProviderRequest::Search(query) => response(
                &request,
                TerminalKind::Success,
                ResponseBody::Search(dispatch_search(query)),
            ),
            ProviderRequest::JumpList(jump_request) => response(
                &request,
                TerminalKind::Success,
                ResponseBody::JumpList(platform_win::common::jump_list::enumerate(
                    &jump_request.application_id,
                    20,
                )),
            ),
            ProviderRequest::Cancel { .. } => unreachable!("cancel returns before dispatch"),
        };
        self.finish(&request.request_id);
        result
    }

    fn enumerate_menu(&mut self, context: &MenuContext) -> MenuEnumeration {
        self.menu_generation = self.menu_generation.saturating_add(1);
        self.menu_tokens.clear();
        let generation = self.menu_generation;
        let mut commands = Vec::new();
        let mut add = |command_id: &str, label: &str, enabled: bool, risk: CommandRisk| {
            let token = format!(
                "ctx:{generation}:{}:{command_id}",
                context.selection_fingerprint
            );
            self.menu_tokens.insert(
                token.clone(),
                RegisteredMenuCommand {
                    generation,
                    selection_fingerprint: context.selection_fingerprint.clone(),
                    command_id: command_id.into(),
                },
            );
            commands.push(CommandDescriptor {
                id: CommandId(token),
                label: label.into(),
                enabled,
                risk,
                children: Vec::new(),
            });
        };
        if context.background {
            add("refresh", "Refresh", true, CommandRisk::Normal);
            add("sort", "Sort by", true, CommandRisk::Normal);
            add("new", "New", true, CommandRisk::Normal);
        } else {
            add("open", "Open", context.can_open, CommandRisk::Normal);
            add(
                "rename",
                "Rename",
                context.can_rename && context.selection_count == 1,
                CommandRisk::Normal,
            );
            add(
                "recycle",
                "Delete",
                context.can_delete,
                CommandRisk::Destructive,
            );
            add(
                "properties",
                "Properties",
                context.can_show_properties,
                CommandRisk::Normal,
            );
        }
        MenuEnumeration {
            generation,
            selection_fingerprint: context.selection_fingerprint.clone(),
            commands,
            optional_enrichment_complete: false,
        }
    }

    fn invoke_menu(&self, invocation: &MenuInvocation) -> Option<MenuInvocationResult> {
        let registered = self.menu_tokens.get(&invocation.token)?;
        (registered.generation == invocation.generation
            && registered.selection_fingerprint == invocation.selection_fingerprint)
            .then(|| MenuInvocationResult {
                command_id: registered.command_id.clone(),
            })
    }
}

fn dispatch_search(query: &SearchQuery) -> Vec<SearchBatch> {
    let mut batches = Vec::new();
    for provider in &query.providers {
        let mut results = match provider {
            SearchProvider::Applications => {
                discover_applications(&default_application_roots(), query.max_results)
            }
            SearchProvider::Settings => settings_catalog(),
            SearchProvider::Files => search_files(
                &query.text,
                &default_file_roots(),
                SearchLimits {
                    max_results: query.max_results,
                    ..SearchLimits::default()
                },
                || true,
            ),
        };
        if !query.text.trim().is_empty() {
            let needle = query.text.to_lowercase();
            results.retain(|result| result.title.to_lowercase().contains(&needle));
        }
        rank_search_results(&query.text, &mut results, &BTreeMap::new());
        results.truncate(query.max_results);
        batches.push(SearchBatch {
            generation: query.generation,
            provider: *provider,
            state: SearchProviderState::Complete,
            results,
        });
    }
    batches
}

fn response(
    request: &Envelope<ProviderRequest>,
    terminal: TerminalKind,
    body: ResponseBody,
) -> ProviderResponse {
    ProviderResponse {
        request_id: request.request_id.clone(),
        correlation_id: request.correlation_id.clone(),
        terminal,
        body,
    }
}

fn terminal_for_validation(
    request: &Envelope<ProviderRequest>,
    error: ValidationError,
) -> ProviderResponse {
    let terminal = match error {
        ValidationError::Expired => TerminalKind::Timeout,
        _ => TerminalKind::InvalidRequest,
    };
    response(request, terminal, ResponseBody::Message(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: &str, payload: ProviderRequest) -> Envelope<ProviderRequest> {
        Envelope {
            protocol: CURRENT_PROTOCOL,
            request_id: id.into(),
            correlation_id: "correlation".into(),
            deadline_unix_ms: Some(2_000),
            payload,
        }
    }

    #[test]
    fn handshake_and_health_report_limits() {
        let mut dispatcher = Dispatcher::new(3);
        let handshake = dispatcher.dispatch(request("one", ProviderRequest::Handshake), 1_000);
        assert_eq!(handshake.terminal, TerminalKind::Success);
        assert!(matches!(
            handshake.body,
            ResponseBody::Handshake(Handshake {
                max_active_requests: 3,
                ..
            })
        ));
        let health = dispatcher.dispatch(request("two", ProviderRequest::Health), 1_000);
        assert!(matches!(
            health.body,
            ResponseBody::Health(HostHealth { healthy: true, .. })
        ));
    }

    #[test]
    fn duplicate_capacity_deadline_and_cancel_are_terminal() {
        let mut dispatcher = Dispatcher::new(1);
        dispatcher.begin("held").unwrap();
        assert_eq!(dispatcher.begin("held"), Err(TerminalKind::InvalidRequest));
        assert_eq!(dispatcher.begin("other"), Err(TerminalKind::Busy));
        let cancel = dispatcher.dispatch(
            request(
                "cancel",
                ProviderRequest::Cancel {
                    target_request_id: "held".into(),
                },
            ),
            1_000,
        );
        assert_eq!(cancel.terminal, TerminalKind::Cancelled);
        let expired = dispatcher.dispatch(request("expired", ProviderRequest::Health), 2_000);
        assert_eq!(expired.terminal, TerminalKind::Timeout);
    }

    #[test]
    fn menu_tokens_are_generation_and_selection_bound() {
        let mut dispatcher = Dispatcher::default();
        let context = MenuContext {
            selection_fingerprint: "one".into(),
            selection_count: 1,
            background: false,
            can_open: true,
            can_rename: true,
            can_delete: true,
            can_show_properties: true,
        };
        let response = dispatcher.dispatch(
            request(
                "menu",
                ProviderRequest::ContextMenuEnumerate(context.clone()),
            ),
            1_000,
        );
        let ResponseBody::Menu(menu) = response.body else {
            panic!()
        };
        let token = menu.commands[0].id.0.clone();
        let valid = dispatcher.dispatch(
            request(
                "invoke",
                ProviderRequest::ContextMenuInvoke(MenuInvocation {
                    generation: menu.generation,
                    selection_fingerprint: "one".into(),
                    token: token.clone(),
                }),
            ),
            1_000,
        );
        assert_eq!(valid.terminal, TerminalKind::Success);
        let stale = dispatcher.dispatch(
            request(
                "stale",
                ProviderRequest::ContextMenuInvoke(MenuInvocation {
                    generation: menu.generation,
                    selection_fingerprint: "other".into(),
                    token,
                }),
            ),
            1_000,
        );
        assert_eq!(stale.terminal, TerminalKind::InvalidRequest);
    }

    #[test]
    fn settings_search_returns_generation_bound_terminal_batch() {
        let mut dispatcher = Dispatcher::default();
        let response = dispatcher.dispatch(
            request(
                "search",
                ProviderRequest::Search(SearchQuery {
                    generation: 9,
                    text: "display".into(),
                    max_results: 10,
                    providers: vec![SearchProvider::Settings],
                }),
            ),
            1_000,
        );
        let ResponseBody::Search(batches) = response.body else {
            panic!()
        };
        assert_eq!(batches[0].generation, 9);
        assert_eq!(batches[0].state, SearchProviderState::Complete);
        assert!(
            batches[0]
                .results
                .iter()
                .any(|result| result.title == "Display settings")
        );
    }

    #[test]
    fn jump_list_is_provider_backed_bounded_and_actionable() {
        let mut dispatcher = Dispatcher::default();
        let application_id = std::env::current_exe()
            .unwrap()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let response = dispatcher.dispatch(
            request(
                "jump-list",
                ProviderRequest::JumpList(shell_provider_protocol::JumpListRequest {
                    application_id,
                }),
            ),
            1_000,
        );
        let ResponseBody::JumpList(list) = response.body else {
            panic!()
        };
        assert!(list.recent.len() <= 20);
        assert_eq!(list.tasks.len(), 1);
        assert!(list.tasks[0].id.0.starts_with("jump:launch:"));
    }
}
