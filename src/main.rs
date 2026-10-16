#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod ui;

fn main() -> iced::Result {
    iced::application(ui::App::default, ui::App::update, ui::App::view)
        .title("HustlX OSS | Basketball Game Day")
        .font(include_bytes!("../assets/fonts/FiraSans-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/FiraSans-Bold.ttf").as_slice())
        .default_font(iced::Font::with_name("Fira Sans"))
        .theme(ui::App::theme)
        .window(iced::window::Settings {
            size: iced::Size::new(1100.0, 760.0),
            min_size: Some(iced::Size::new(1100.0, 760.0)),
            ..Default::default()
        })
        .run()
}
