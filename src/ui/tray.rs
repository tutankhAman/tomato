use ksni::{MenuItem, Tray, TrayService};
use std::sync::mpsc::Sender;

pub enum TrayAction {
    Toggle,
    Quit,
}

struct TomatoTray {
    sender: Sender<TrayAction>,
}

impl Tray for TomatoTray {
    fn id(&self) -> String {
        "dev.aamn.tomato".into()
    }

    fn title(&self) -> String {
        "Tomato".into()
    }

    fn icon_name(&self) -> String {
        "dev.aamn.tomato".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "Tomato".into(),
            description: "Sticky Pomodoro & Todo Tracker".into(),
            icon_name: "dev.aamn.tomato".into(),
            icon_pixmap: Vec::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(TrayAction::Toggle);
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::ApplicationStatus
    }

    fn status(&self) -> ksni::Status {
        ksni::Status::Active
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Toggle Panel".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.sender.send(TrayAction::Toggle);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit Tomato".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.sender.send(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn(sender: Sender<TrayAction>) {
    let tray = TomatoTray { sender };
    let service = TrayService::new(tray);
    service.spawn();
}
