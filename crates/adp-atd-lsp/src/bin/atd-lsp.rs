//! `atd-lsp`, the ATD language server over stdio (etalii.adp spec 035, research R10), as IntelliJ's platform LSP API
//! starts it.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    adp_atd_lsp::transport::run_stdio();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
