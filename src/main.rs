#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod brand;
mod native_quit;
mod ui;

fn main() -> iced::Result {
    iced::application(ui::App::default, ui::App::update, ui::App::view)
        .title("HustlX OSS | Offline Game Day")
        .font(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Exo-Bold.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Exo-Black.ttf").as_slice())
        .default_font(brand::BODY)
        .theme(ui::App::theme)
        .subscription(ui::App::subscription)
        .window(iced::window::Settings {
            size: iced::Size::new(1100.0, 760.0),
            min_size: Some(iced::Size::new(1100.0, 760.0)),
            exit_on_close_request: false,
            ..Default::default()
        })
        .run()
}
