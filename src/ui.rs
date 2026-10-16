use hustlx_oss::{Preview, Side, Stat};
use iced::widget::{Column, Container, Space, button, column, container, row, scrollable, text};
use iced::{Border, Color, Element, Fill, Font, Length, Theme, alignment};

const HOME: Color = Color::from_rgb(0.62, 0.57, 1.0);
const AWAY: Color = Color::from_rgb(0.96, 0.81, 0.25);
const GREEN: Color = Color::from_rgb(0.40, 0.80, 0.64);
const RED: Color = Color::from_rgb(0.98, 0.42, 0.49);
const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::with_name("Fira Sans")
};

#[derive(Debug, Clone)]
pub enum Message {
    Select(Side, u32),
    Record(Stat),
    Undo,
    ToggleTheme,
}

pub struct App {
    game: Preview,
    dark: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            game: Preview::default(),
            dark: true,
        }
    }
}

#[derive(Clone, Copy)]
struct Colors {
    background: Color,
    panel: Color,
    row: Color,
    border: Color,
    text: Color,
    muted: Color,
    home: Color,
    away: Color,
    green: Color,
    red: Color,
}

impl App {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Select(side, id) => self.game.select(side, id),
            Message::Record(stat) => {
                self.game.record(stat);
            }
            Message::Undo => {
                self.game.undo();
            }
            Message::ToggleTheme => self.dark = !self.dark,
        }
    }

    pub fn theme(&self) -> Theme {
        if self.dark { Theme::Dark } else { Theme::Light }
    }

    fn colors(&self) -> Colors {
        if self.dark {
            Colors {
                background: rgb(16, 17, 20),
                panel: rgb(24, 25, 29),
                row: rgb(32, 33, 38),
                border: rgb(48, 49, 57),
                text: rgb(242, 243, 247),
                muted: rgb(150, 154, 167),
                home: HOME,
                away: AWAY,
                green: GREEN,
                red: RED,
            }
        } else {
            Colors {
                background: rgb(241, 243, 248),
                panel: Color::WHITE,
                row: rgb(247, 248, 251),
                border: rgb(219, 223, 232),
                text: rgb(25, 28, 41),
                muted: rgb(99, 106, 122),
                home: rgb(83, 67, 172),
                away: rgb(130, 102, 16),
                green: rgb(36, 113, 83),
                red: rgb(177, 50, 67),
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let c = self.colors();
        let header = row![
            text("HUSTLX").size(19).font(BOLD),
            text("/  GAME DAY").size(12).color(c.muted),
            Space::new().width(Fill),
            badge("NATIVE RUST PREVIEW", c.green, c),
            button(
                text(if self.dark {
                    "Light theme"
                } else {
                    "Dark theme"
                })
                .size(12)
            )
            .on_press(Message::ToggleTheme)
            .padding([8, 14])
            .style(button::secondary),
        ]
        .spacing(14)
        .align_y(alignment::Vertical::Center);

        let scoreboard = card(
            column![
                row![
                    text("SAMPLE MATCH").size(10).color(c.muted),
                    Space::new().width(Fill),
                    text("BASKETBALL / 5v5").size(10).color(c.muted)
                ],
                row![
                    self.score(Side::Home, c),
                    self.clock(c),
                    self.score(Side::Away, c)
                ]
                .spacing(18)
                .align_y(alignment::Vertical::Center),
            ]
            .spacing(6),
            c,
        )
        .padding([16, 26]);

        let body = row![
            self.roster(Side::Home, c),
            self.command_center(c),
            self.roster(Side::Away, c),
        ]
        .spacing(14)
        .height(Fill);

        let notice = row![
            text("SAMPLE DATA").font(BOLD).size(10).color(c.away),
            text("Changes are not saved. Clock is a fixed preview.")
                .size(12)
                .color(c.muted),
            Space::new().width(Fill),
            text("LOCAL TEAMS + MATCHES COMING NEXT")
                .size(10)
                .color(c.muted),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center);

        container(column![header, scoreboard, body, notice].spacing(16))
            .padding(20)
            .width(Fill)
            .height(Fill)
            .style(move |_| {
                container::Style::default()
                    .background(c.background)
                    .color(c.text)
            })
            .into()
    }

    fn score(&self, side: Side, c: Colors) -> Element<'_, Message> {
        let accent = if side == Side::Home { c.home } else { c.away };
        let fouls: u32 = self
            .game
            .team(side)
            .players
            .iter()
            .map(|p| self.game.player_stats(side, p.id).fouls)
            .sum();
        column![
            text(self.game.team(side).name.to_uppercase())
                .size(13)
                .font(BOLD)
                .color(accent),
            text(format!("{:02}", self.game.score(side)))
                .size(68)
                .font(BOLD),
            text(format!("{fouls} personal fouls"))
                .size(12)
                .color(c.muted),
        ]
        .width(Length::FillPortion(3))
        .align_x(if side == Side::Home {
            alignment::Horizontal::Left
        } else {
            alignment::Horizontal::Right
        })
        .into()
    }

    fn clock(&self, c: Colors) -> Element<'_, Message> {
        column![
            text("Q3  /  CLOCK PREVIEW").size(10).color(c.muted),
            row![
                column![
                    text("17").size(32).font(BOLD).color(c.away),
                    text("SHOT").size(9).color(c.muted)
                ]
                .align_x(alignment::Horizontal::Center),
                text("04:12").size(60).font(BOLD),
            ]
            .spacing(14)
            .align_y(alignment::Vertical::Center),
            text("Game and shot clock controls are planned")
                .size(11)
                .color(c.muted),
        ]
        .spacing(4)
        .align_x(alignment::Horizontal::Center)
        .width(Length::FillPortion(4))
        .into()
    }

    fn roster(&self, side: Side, c: Colors) -> Element<'_, Message> {
        let team = self.game.team(side);
        let accent = if side == Side::Home { c.home } else { c.away };
        let heading = row![
            text(&team.name).size(16).font(BOLD),
            Space::new().width(Fill),
            text(if side == Side::Home { "HOME" } else { "AWAY" })
                .size(10)
                .color(accent)
        ]
        .align_y(alignment::Vertical::Center);
        let mut players = Column::new().spacing(8);
        for on_court in [true, false] {
            players = players.push(
                container(
                    text(if on_court { "ON COURT" } else { "BENCH" })
                        .size(10)
                        .color(c.muted),
                )
                .padding([8, 2]),
            );
            for p in team.players.iter().filter(|p| p.on_court == on_court) {
                let stats = self.game.player_stats(side, p.id);
                let selected = self.game.selected == Some((side, p.id));
                let content = row![
                    text(format!("{:02}", p.jersey))
                        .size(25)
                        .font(BOLD)
                        .color(accent)
                        .width(38),
                    column![
                        text(&p.name).size(14).font(BOLD),
                        text(format!(
                            "{} AST  /  {} REB",
                            stats.assists,
                            stats.offensive_rebounds + stats.defensive_rebounds
                        ))
                        .size(10)
                        .color(c.muted),
                    ]
                    .spacing(4)
                    .width(Fill),
                    text(stats.points).size(22).font(BOLD),
                    text(format!("PF {}", stats.fouls)).size(10).color(c.muted),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center);
                players = players.push(
                    button(content)
                        .on_press(Message::Select(side, p.id))
                        .width(Fill)
                        .padding([12, 10])
                        .style(move |_, status| {
                            let active = selected || status == button::Status::Hovered;
                            button::Style {
                                background: Some(
                                    if active {
                                        blend(c.row, accent, 0.15)
                                    } else {
                                        c.row
                                    }
                                    .into(),
                                ),
                                text_color: c.text,
                                border: Border {
                                    color: if selected { accent } else { c.border },
                                    width: if selected { 1.5 } else { 1.0 },
                                    radius: 10.0.into(),
                                },
                                ..Default::default()
                            }
                        }),
                );
            }
        }
        card(
            column![
                heading,
                scrollable(players.width(Fill))
                    .width(Fill)
                    .spacing(8)
                    .height(Fill)
            ]
            .spacing(14),
            c,
        )
        .width(Length::FillPortion(3))
        .height(Fill)
        .into()
    }

    fn command_center(&self, c: Colors) -> Element<'_, Message> {
        let target = self
            .game
            .selected
            .and_then(|(side, id)| self.game.player(side, id));
        let title = match target {
            Some(player) => format!("Pad aimed at #{} {}", player.jersey, player.name),
            None => "Select a player from either roster".to_owned(),
        };
        let pad = column![
            text(title).size(13).color(c.muted),
            row![
                self.stat_button(Stat::FreeThrowMade, c.green, c),
                self.stat_button(Stat::TwoMade, c.green, c),
                self.stat_button(Stat::ThreeMade, c.green, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::FreeThrowMiss, c.red, c),
                self.stat_button(Stat::TwoMiss, c.red, c),
                self.stat_button(Stat::ThreeMiss, c.red, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::OffensiveRebound, c.away, c),
                self.stat_button(Stat::DefensiveRebound, c.green, c),
                self.stat_button(Stat::Assist, c.home, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::Turnover, c.away, c),
                self.stat_button(Stat::Steal, c.home, c),
                self.stat_button(Stat::Block, c.home, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::Foul, c.red, c),
                self.stat_button(Stat::Technical, c.red, c),
                self.stat_button(Stat::Warning, c.muted, c)
            ]
            .spacing(6),
            row![
                self.stat_button(Stat::Unsportsmanlike, c.red, c),
                self.stat_button(Stat::Disqualifying, c.red, c)
            ]
            .spacing(6),
        ]
        .spacing(7);

        let undo = button(text("Undo last").size(11))
            .padding([7, 12])
            .on_press_maybe((!self.game.plays().is_empty()).then_some(Message::Undo))
            .style(button::secondary);
        let mut feed = Column::new().spacing(8);
        if self.game.plays().is_empty() {
            feed = feed.push(container(column![
                text("Ready for the first play").size(18).font(BOLD),
                text("Select a player, then record a shot or stat.\nYour sample plays will appear here.").size(13).color(c.muted),
            ].spacing(10)).padding([24, 8]));
        }
        for (index, play) in self.game.plays().iter().enumerate().rev() {
            let player = self
                .game
                .player(play.side, play.player_id)
                .expect("preview plays contain known players");
            let accent = if play.side == Side::Home {
                c.home
            } else {
                c.away
            };
            feed = feed.push(
                container(
                    row![
                        text(format!("{:02}", index + 1))
                            .size(12)
                            .color(c.muted)
                            .width(26),
                        column![
                            text(play.stat.label()).size(12).font(BOLD).color(accent),
                            text(format!("#{} {}", player.jersey, player.name)).size(13),
                        ]
                        .spacing(5)
                        .width(Fill),
                        text(if play.stat.points() > 0 {
                            format!("+{}", play.stat.points())
                        } else {
                            String::new()
                        })
                        .size(17)
                        .font(BOLD)
                        .color(accent),
                    ]
                    .spacing(8)
                    .align_y(alignment::Vertical::Center),
                )
                .padding(12)
                .width(Fill)
                .style(move |_| {
                    container::Style::default()
                        .background(c.row)
                        .border(Border {
                            color: c.border,
                            width: 1.0,
                            radius: 10.0.into(),
                        })
                }),
            );
        }
        card(
            column![
                pad,
                row![
                    text(format!("PLAY-BY-PLAY  /  {}", self.game.plays().len()))
                        .size(10)
                        .color(c.muted),
                    Space::new().width(Fill),
                    undo
                ]
                .align_y(alignment::Vertical::Center),
                scrollable(feed.width(Fill))
                    .width(Fill)
                    .spacing(8)
                    .height(Fill),
            ]
            .spacing(16),
            c,
        )
        .width(Length::FillPortion(4))
        .height(Fill)
        .into()
    }

    fn stat_button(&self, stat: Stat, accent: Color, c: Colors) -> Element<'_, Message> {
        button(
            text(stat.label())
                .size(12)
                .font(BOLD)
                .width(Fill)
                .align_x(alignment::Horizontal::Center),
        )
        .on_press_maybe(self.game.selected.map(|_| Message::Record(stat)))
        .padding([13, 4])
        .width(Fill)
        .style(move |_, status| {
            let disabled = status == button::Status::Disabled;
            button::Style {
                background: Some(
                    blend(
                        c.panel,
                        accent,
                        if disabled {
                            0.06
                        } else if status == button::Status::Hovered {
                            0.28
                        } else {
                            0.16
                        },
                    )
                    .into(),
                ),
                text_color: if disabled { c.muted } else { accent },
                border: Border {
                    color: blend(c.border, accent, if disabled { 0.08 } else { 0.3 }),
                    width: 1.0,
                    radius: 7.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }
}

fn card<'a>(content: impl Into<Element<'a, Message>>, c: Colors) -> Container<'a, Message> {
    container(content).padding(14).style(move |_| {
        container::Style::default()
            .background(c.panel)
            .color(c.text)
            .border(Border {
                color: c.border,
                width: 1.0,
                radius: 14.0.into(),
            })
    })
}

fn badge<'a>(label: &'a str, accent: Color, c: Colors) -> Element<'a, Message> {
    container(text(label).size(10).font(BOLD).color(accent))
        .padding([6, 10])
        .style(move |_| {
            container::Style::default()
                .background(blend(c.panel, accent, 0.12))
                .border(Border {
                    color: blend(c.border, accent, 0.3),
                    width: 1.0,
                    radius: 20.0.into(),
                })
        })
        .into()
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn blend(base: Color, accent: Color, amount: f32) -> Color {
    Color::from_rgb(
        base.r + (accent.r - base.r) * amount,
        base.g + (accent.g - base.g) * amount,
        base.b + (accent.b - base.b) * amount,
    )
}
