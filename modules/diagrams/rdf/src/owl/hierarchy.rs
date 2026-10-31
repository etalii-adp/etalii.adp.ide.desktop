//! The asserted subclass graph's shape (etalii.adp spec 030, task T083), shared by the layout (column depths) and the
//! validator (cycle warnings): moved to the capability `layout.depthColumns` (etalii.adp spec 067, T040).

pub use adp_atd_runtime::capabilities::depth_columns::{Supers, cycles, depths};
