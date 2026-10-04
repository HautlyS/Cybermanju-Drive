#[cfg(test)]
mod compression;
#[cfg(test)]
mod crypto;
#[cfg(test)]
mod db;
#[cfg(test)]
mod search;
#[cfg(test)]
mod types;
#[cfg(test)]
mod web;
// <<< AGENT-4 item 9 — REST/auth suites, one module per route family >>>
#[cfg(test)]
mod api_users;
#[cfg(test)]
mod share;
#[cfg(test)]
mod sync_routes;
// <<< AGENT-4 item 10 — provider contracts against a local mock server >>>
#[cfg(test)]
mod backends_contract;
