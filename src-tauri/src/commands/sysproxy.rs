//! Only `sysproxy_status` is exposed to the frontend — enable/disable is
//! driven server-side by `core_start`/`core_stop` so that sysproxy and the
//! runner cannot drift apart (e.g. sysproxy left pointing at a dead port
//! after Disconnect).

use crate::sysproxy::SysProxyState;
use tauri::State;

#[tauri::command]
pub async fn sysproxy_status(sys: State<'_, SysProxyState>) -> Result<bool, String> {
    Ok(sys.0.lock().await.is_some())
}
