use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

use gpui::{BackgroundExecutor, ForegroundExecutor, Task};
use openharmony_ability::{FrameInputDelivery, OpenHarmonyApp};

use super::{
    frame_demand::{DeliveryChange, FrameDemand},
    window::OhosWindow,
};

const FALLBACK_FRAME_DELAY: Duration = Duration::from_millis(16);

pub(crate) struct FrameCallbackScope {
    active: Rc<Cell<bool>>,
    previous: bool,
}

impl FrameCallbackScope {
    pub(crate) fn enter(active: Rc<Cell<bool>>) -> Self {
        let previous = active.replace(true);
        Self { active, previous }
    }
}

impl Drop for FrameCallbackScope {
    fn drop(&mut self) {
        self.active.set(self.previous);
    }
}

pub(crate) struct WindowFrames {
    app: Rc<RefCell<Option<OpenHarmonyApp>>>,
    window_id: i64,
    window: RefCell<Weak<RefCell<OhosWindow>>>,
    demand: RefCell<FrameDemand>,
    surface_available: Cell<bool>,
    foreground: ForegroundExecutor,
    background: BackgroundExecutor,
    sync_queued: Cell<bool>,
    callback_active: Rc<Cell<bool>>,
    fallback_task: RefCell<Option<Task<()>>>,
    register_warned: Cell<bool>,
    unregister_warned: Cell<bool>,
}

impl WindowFrames {
    pub(crate) fn new(
        window_id: i64,
        app: Rc<RefCell<Option<OpenHarmonyApp>>>,
        foreground: ForegroundExecutor,
        background: BackgroundExecutor,
        callback_active: Rc<Cell<bool>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            app,
            window_id,
            window: RefCell::new(Weak::new()),
            demand: RefCell::new(FrameDemand::default()),
            surface_available: Cell::new(false),
            foreground,
            background,
            sync_queued: Cell::new(false),
            callback_active,
            fallback_task: RefCell::new(None),
            register_warned: Cell::new(false),
            unregister_warned: Cell::new(false),
        })
    }

    pub(crate) fn set_window(&self, window: Weak<RefCell<OhosWindow>>) {
        *self.window.borrow_mut() = window;
    }

    pub(crate) fn request(self: &Rc<Self>) {
        self.demand.borrow_mut().request();
        self.sync_frame_delivery();
    }

    pub(crate) fn set_active(self: &Rc<Self>, visible: bool, surface_available: bool) {
        if self.surface_available.replace(surface_available) && !surface_available {
            self.demand.borrow_mut().surface_destroyed();
        }
        self.demand
            .borrow_mut()
            .set_active(visible && surface_available);
        if !visible || !surface_available {
            self.fallback_task.borrow_mut().take();
        }
        self.sync_frame_delivery();
    }

    fn sync_frame_delivery(self: &Rc<Self>) {
        let Some(change) = self.demand.borrow().pending() else {
            return;
        };
        if self.callback_active.get() {
            self.defer_sync();
            return;
        }
        let Some(app) = self.app.borrow().clone() else {
            return;
        };
        let register = change == DeliveryChange::Register;
        let delivery = if register {
            FrameInputDelivery::Continuous
        } else {
            FrameInputDelivery::OnDemand
        };
        let result = app.set_frame_input_delivery_for(self.window_id, delivery);
        self.demand.borrow_mut().complete(change, result.is_ok());
        if let Err(error) = result {
            let warned = if register {
                &self.register_warned
            } else {
                &self.unregister_warned
            };
            if !warned.replace(true) {
                let operation = if register { "register" } else { "unregister" };
                log::warn!(
                    "Cannot {operation} OHOS frame callback for window {}: {error}",
                    self.window_id
                );
            }
            if register {
                self.schedule_fallback();
            }
        } else if register {
            self.fallback_task.borrow_mut().take();
        }
    }

    fn schedule_fallback(self: &Rc<Self>) {
        if !self.demand.borrow_mut().schedule_fallback() {
            return;
        }
        let frames = Rc::downgrade(self);
        let background = self.background.clone();
        let task = self.foreground.spawn(async move {
            loop {
                background.timer(FALLBACK_FRAME_DELAY).await;
                let Some(frames) = frames.upgrade() else {
                    return;
                };
                if frames.callback_active.get() {
                    continue;
                }
                // A draw may request another frame and replace this task.
                if let Some(task) = frames.fallback_task.borrow_mut().take() {
                    task.detach();
                }
                let draw = frames.demand.borrow_mut().begin_fallback();
                if draw && let Some(window) = frames.window.borrow().upgrade() {
                    window.borrow().draw_requested_frame(Instant::now());
                }
                return;
            }
        });
        *self.fallback_task.borrow_mut() = Some(task);
    }

    fn defer_sync(self: &Rc<Self>) {
        if self.sync_queued.replace(true) {
            return;
        }
        let frames = self.clone();
        // Keep cleanup alive through window drop; redraw events never drain this queue.
        self.foreground
            .spawn(async move {
                frames.sync_queued.set(false);
                frames.sync_frame_delivery();
            })
            .detach();
    }

    pub(crate) fn on_callback(self: &Rc<Self>) {
        self.fallback_task.borrow_mut().take();
        if self.demand.borrow_mut().on_callback().is_some() {
            self.defer_sync();
        }
    }
}
