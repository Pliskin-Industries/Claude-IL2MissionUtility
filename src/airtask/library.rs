//! Air sortie library; loading and inspection are built in step 2.

use super::AirSortie;

pub fn builtin_library() -> Vec<AirSortie> {
    vec![]
}

pub fn load_user_sortie(_path: &std::path::Path) -> Result<AirSortie, String> {
    Err("load_user_sortie is built in step 2".to_string())
}
