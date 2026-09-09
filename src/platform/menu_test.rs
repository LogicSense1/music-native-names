use super::*;

pub(super) static mut MAIN_MENU: Id = ptr::null_mut();
static mut SELF_TEST_STAGE: u32 = 0;

unsafe fn menu_snapshot(menu: Id) -> serde_json::Value {
    let count: usize = msg0(menu, "numberOfItems");
    let mut rows = Vec::new();
    for i in 0..count {
        let item: Id = msg1(menu, "itemAtIndex:", i as isize);
        let sub: Id = msg0(item, "submenu");
        let t: Id = msg0(item, "title");
        let state: isize = msg0(item, "state");
        let enabled: bool = msg0(item, "isEnabled");
        let tag: isize = msg0(item, "tag");
        rows.push(serde_json::json!({"title":text(t),"state":state,"enabled":enabled,"tag":tag,"submenu":if sub.is_null(){serde_json::Value::Null}else{menu_snapshot(sub)}}));
    }
    serde_json::json!(rows)
}
unsafe fn perform(item: Id) {
    let menu: Id = msg0(item, "menu");
    let index: isize = msg1(menu, "indexOfItem:", item);
    msg1::<_, ()>(menu, "performActionForItemAtIndex:", index);
}
pub(super) unsafe fn menu_self_test(app: &Arc<App>) {
    if !std::env::args().any(|a| a == "--no-hooks") {
        return;
    }
    SELF_TEST_STAGE += 1;
    match SELF_TEST_STAGE {
        1 => {
            let _ = crate::atomic_json(
                &app.state.join("menu-initial.json"),
                &menu_snapshot(MAIN_MENU),
            );
            perform(RETENTION[0]);
        }
        2 => {
            if app.prefs.lock().unwrap().retention_days != 1 {
                std::process::exit(2);
            }
            perform(RETENTION[1]);
        }
        3 => {
            if app.prefs.lock().unwrap().retention_days != 7 {
                std::process::exit(3);
            }
            perform(ENABLE);
        }
        4 => {
            if app.enabled() {
                std::process::exit(4);
            }
            perform(ENABLE);
        }
        5 => {
            if !app.enabled() {
                std::process::exit(5);
            }
            let _ = crate::atomic_json(
                &app.state.join("menu-tested.json"),
                &menu_snapshot(MAIN_MENU),
            );
            app.log("menu_self_test_passed",serde_json::json!({"retentionActions":true,"pauseResumeActions":true,"nativeMenuDispatch":true}));
            let quit: Id = msg1(MAIN_MENU, "itemWithTag:", 7isize);
            perform(quit);
        }
        _ => {}
    }
}
