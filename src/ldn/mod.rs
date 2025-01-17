pub mod latency_slider;
pub mod net;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::framerate;
use crate::ldn::net::interface::{get_network_role, NetworkRole};
use crate::utils::TextBoxExt;
use skyline::hooks::InlineCtx;
use skyline::nn::ui2d::Pane;

static mut CUSTOM_CSS_NUM_PLAYERS_FLAG: bool = false;

static LOCAL_ROOM_PANE_HANDLE: AtomicU64 = AtomicU64::new(0);
// workaround for sv_information::is_ready_go() being unreliable for ldn in some cases
static IN_GAME: AtomicBool = AtomicBool::new(false);

#[skyline::hook(offset = 0x22d9d10, inline)]
unsafe fn online_melee_any_scene_create(_: &InlineCtx) {
    LOCAL_ROOM_PANE_HANDLE.store(0, Ordering::SeqCst);
    framerate::set_framerate_target(60);
    framerate::set_vsync_enabled(true);
}

#[skyline::hook(offset = 0x22d9c40, inline)]
unsafe fn bg_matchmaking_seq(_: &InlineCtx) {
    LOCAL_ROOM_PANE_HANDLE.store(0, Ordering::SeqCst);
    framerate::set_framerate_target(60);
    framerate::set_vsync_enabled(true);
}

#[skyline::hook(offset = 0x235a650, inline)]
unsafe fn main_menu(_: &InlineCtx) {
    LOCAL_ROOM_PANE_HANDLE.store(0, Ordering::SeqCst);
    framerate::set_framerate_target(60);
    framerate::set_vsync_enabled(true);
}

// called on local online menu init
#[skyline::hook(offset = 0x1bd45e0, inline)]
unsafe fn store_local_menu_pane(ctx: &InlineCtx) {
    update_in_game_flag(false);
    CUSTOM_CSS_NUM_PLAYERS_FLAG = true;
    let handle = *((*((*ctx.registers[0].x.as_ref() + 8) as *const u64) + 0x10) as *const u64);
    LOCAL_ROOM_PANE_HANDLE.store(handle, Ordering::SeqCst);
}

#[skyline::hook(offset = 0x1bd7a80, inline)]
unsafe fn update_local_menu(_: &InlineCtx) {
    let pane_handle = LOCAL_ROOM_PANE_HANDLE.load(Ordering::SeqCst) as *mut u64 as *mut Pane;
    if !pane_handle.is_null() {
        latency_slider::poll();
        let delay_str = latency_slider::current_input_delay().to_string();
        (*pane_handle)
            .as_textbox()
            .set_text_string(&format!("{}", delay_str));
    }
}

#[skyline::hook(offset = 0x1a26200)]
unsafe fn css_player_pane_num_changed(param_1: i64, prev_num: i32, changed_by_player: u32) {
    if is_local_online()
        && CUSTOM_CSS_NUM_PLAYERS_FLAG
        && changed_by_player == 0
        && get_network_role() == NetworkRole::Host
    {
        CUSTOM_CSS_NUM_PLAYERS_FLAG = false;
        *((param_1 + 0x160) as *mut i32) = 2;
    }
    call_original!(param_1, prev_num, changed_by_player);
}

#[skyline::hook(offset = 0x1345558, inline)]
unsafe fn on_match_start(_: &InlineCtx) {
    if !is_local_online() {
        return;
    }
    update_in_game_flag(true);
}

#[skyline::hook(offset = 0x1d68b94, inline)]
unsafe fn on_match_end(_: &InlineCtx) {
    if !is_local_online() {
        return;
    }
    update_in_game_flag(false);
}

fn update_in_game_flag(new_in_game_flag: bool) {
    let _ = IN_GAME.compare_exchange(
        !new_in_game_flag,
        new_in_game_flag,
        Ordering::SeqCst,
        Ordering::SeqCst,
    );
}

pub fn is_local_online() -> bool {
    return LOCAL_ROOM_PANE_HANDLE.load(Ordering::SeqCst) > 0;
}

pub fn is_in_game() -> bool {
    IN_GAME.load(Ordering::SeqCst)
}

pub fn install() {
    skyline::install_hooks!(
        online_melee_any_scene_create,
        bg_matchmaking_seq,
        main_menu,
        store_local_menu_pane,
        update_local_menu,
        css_player_pane_num_changed,
        on_match_start,
        on_match_end,
    );
    latency_slider::install();
    net::install();
}
