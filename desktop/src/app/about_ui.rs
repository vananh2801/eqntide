//! Desktop About view and on-demand update controls.

use super::App;
use crate::{
    theme::{self, ThemeTokens},
    ui::{self, icons::UI_ICON_SIZE},
    update::{self, UpdateState},
    version::app_version,
};
use eframe::egui::{self, Vec2};

impl App {
    /// Renders the About tab showing version, logo, author, license, repository link, and manual updates.
    pub(super) fn draw_about_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = self.theme_tokens(ui);
        self.draw_back_to_editor_button(ui);
        let author_icon = self
            .ui_icons
            .image(ui.ctx(), "user", UI_ICON_SIZE, tokens.text_muted);
        let license_icon =
            self.ui_icons
                .image(ui.ctx(), "license", UI_ICON_SIZE, tokens.text_muted);
        let github_icon =
            self.ui_icons
                .image(ui.ctx(), "brand-github", UI_ICON_SIZE, tokens.text_muted);
        let logo_id = self
            .logo_texture
            .get_or_insert_with(|| {
                let bytes = include_bytes!("../../../assets/brand/icon.png");
                let pixels = image::load_from_memory(bytes)
                    .expect("bundled icon is valid")
                    .to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [pixels.width() as usize, pixels.height() as usize],
                    pixels.as_raw(),
                );
                ui.ctx()
                    .load_texture("eqntide-logo", image, egui::TextureOptions::LINEAR)
            })
            .id();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui::widgets::full_width_card(ui, tokens, |ui| {
                    const ABOUT_LOGO_SIZE: f32 = 72.0;
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Image::new((logo_id, Vec2::splat(ABOUT_LOGO_SIZE)))
                                .corner_radius(theme::SURFACE_CORNER_RADIUS),
                        );
                        ui.add_space(theme::SPACE_LG);
                        let title = format!("EqnTide {}", app_version());
                        let desc = env!("CARGO_PKG_DESCRIPTION");
                        let desc = desc.strip_prefix("EqnTide: ").unwrap_or(desc);
                        let avail_w = ui.available_width().max(1.0);
                        let heading_font = egui::TextStyle::Heading.resolve(ui.style());
                        let body_font = egui::TextStyle::Body.resolve(ui.style());
                        let title_h = ui.fonts_mut(|fonts| {
                            fonts
                                .layout(title.clone(), heading_font, tokens.text_primary, avail_w)
                                .size()
                                .y
                        });
                        let desc_h = ui.fonts_mut(|fonts| {
                            fonts
                                .layout(desc.to_owned(), body_font, tokens.text_primary, avail_w)
                                .size()
                                .y
                        });
                        let content_h = title_h + ui.spacing().item_spacing.y + desc_h;
                        let top_pad = ((ABOUT_LOGO_SIZE - content_h) * 0.5).max(0.0);
                        ui.vertical(|ui| {
                            ui.add_space(top_pad);
                            ui.heading(title);
                            ui.label(desc);
                        });
                    });
                    ui.add_space(theme::SPACE_XL);
                    ui::widgets::section_separator(ui, tokens);
                    ui.add_space(theme::SPACE_MD);
                    egui::Grid::new("about-info")
                        .num_columns(2)
                        .spacing([theme::SPACE_XL, theme::SPACE_MD])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(author_icon);
                                ui.strong("Author:");
                            });
                            ui.label(env!("CARGO_PKG_AUTHORS"));
                            ui.end_row();

                            ui.horizontal(|ui| {
                                ui.add(license_icon);
                                ui.strong("License:");
                            });
                            ui.label(env!("CARGO_PKG_LICENSE"));
                            ui.end_row();

                            ui.horizontal(|ui| {
                                ui.add(github_icon);
                                ui.strong("GitHub:");
                            });
                            ui.hyperlink(env!("CARGO_PKG_REPOSITORY"));
                            ui.end_row();
                        });
                    ui.add_space(theme::SPACE_XL);
                    ui::widgets::section_separator(ui, tokens);
                    ui.add_space(theme::SPACE_MD);
                    self.draw_about_updates(ui, tokens);
                });
            });
    }

    fn draw_about_updates(&mut self, ui: &mut egui::Ui, tokens: ThemeTokens) {
        ui.strong("Updates");
        ui.add_space(theme::SPACE_XS);
        match self.update_state.clone() {
            UpdateState::Idle => {
                if ui::widgets::standard_button(ui, "Check for updates").clicked() {
                    self.start_update_check();
                }
            }
            UpdateState::Checking => {
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(false, |ui| {
                        ui::widgets::standard_button(ui, "Check for updates")
                    });
                    ui.label(egui::RichText::new("Checking for updates…").color(tokens.text_muted));
                });
            }
            UpdateState::UpToDate => {
                ui.horizontal(|ui| {
                    if ui::widgets::standard_button(ui, "Check again").clicked() {
                        self.start_update_check();
                    }
                    ui.label(
                        egui::RichText::new(format!("EqnTide {} is up to date.", app_version()))
                            .color(tokens.text_muted),
                    );
                });
            }
            UpdateState::Available(release) => {
                ui.label(
                    egui::RichText::new(format!("Update available: v{}", release.version))
                        .strong()
                        .color(tokens.accent),
                );
                ui.add_space(theme::SPACE_XS);
                ui.horizontal(|ui| {
                    let button_label = if cfg!(target_os = "windows")
                        && release.download_url != release.release_url
                    {
                        format!("Download v{}", release.version)
                    } else {
                        format!("Open release v{}", release.version)
                    };
                    if ui::widgets::primary_button_with(ui, tokens, true, |ui| {
                        ui.add(
                            egui::Button::new(button_label)
                                .min_size(Vec2::new(0.0, theme::BUTTON_HEIGHT)),
                        )
                    })
                    .clicked()
                        && update::is_trusted_release_url(&release.download_url)
                    {
                        ui.ctx()
                            .open_url(egui::OpenUrl::new_tab(release.download_url.clone()));
                    }
                    if release.download_url != release.release_url
                        && ui::widgets::standard_button(ui, "Release notes").clicked()
                        && update::is_trusted_release_url(&release.release_url)
                    {
                        ui.ctx()
                            .open_url(egui::OpenUrl::new_tab(release.release_url.clone()));
                    }
                    if ui::widgets::standard_button(ui, "Check again").clicked() {
                        self.start_update_check();
                    }
                });
            }
            UpdateState::Error(err) => {
                ui.label(egui::RichText::new(err).small().color(tokens.error_fg));
                ui.add_space(theme::SPACE_XS);
                ui.horizontal(|ui| {
                    if ui::widgets::standard_button(ui, "Retry").clicked() {
                        self.start_update_check();
                    }
                    if ui::widgets::standard_button(ui, "Open Releases").clicked() {
                        ui.ctx()
                            .open_url(egui::OpenUrl::new_tab(update::GITHUB_RELEASES_PAGE_URL));
                    }
                });
            }
        }
    }
}
