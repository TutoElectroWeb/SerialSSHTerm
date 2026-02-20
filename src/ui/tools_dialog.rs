// =============================================================================
// Fichier : tools_dialog.rs
// Rôle    : Fenêtre d'outils (calculatrice + convertisseur de base)
// =============================================================================

use anyhow::Context;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DropDown, Entry, Label, Orientation, StringList};

#[allow(clippy::too_many_lines)]
pub fn open_tools_dialog(parent: &impl IsA<gtk4::Window>) {
    let dialog = gtk4::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Outils")
        .default_width(520)
        .default_height(320)
        .build();

    let content = GtkBox::builder().orientation(Orientation::Vertical).build();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    // ---------------------------------------------------------------------
    // Calculatrice
    // ---------------------------------------------------------------------
    let calc_title = Label::builder().label("Calculatrice").xalign(0.0).build();
    let calc_box = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();
    let calc_entry = Entry::builder()
        .placeholder_text("Ex: (12+5)*3/2")
        .hexpand(true)
        .build();
    let calc_button = Button::builder().label("Calculer").build();
    calc_box.append(&calc_entry);
    calc_box.append(&calc_button);
    let calc_result = Label::builder().label("Résultat: -").xalign(0.0).build();

    // ---------------------------------------------------------------------
    // Convertisseur DEC/HEX/BIN
    // ---------------------------------------------------------------------
    let conv_title = Label::builder()
        .label("Convertisseur DEC / HEX / BIN")
        .xalign(0.0)
        .build();
    let conv_row = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(8)
        .build();

    let base_model = StringList::new(&["DEC", "HEX", "BIN"]);
    let base_dropdown = DropDown::builder().model(&base_model).selected(0).build();

    let value_entry = Entry::builder()
        .placeholder_text("Valeur à convertir")
        .hexpand(true)
        .build();
    let convert_button = Button::builder().label("Convertir").build();

    conv_row.append(&base_dropdown);
    conv_row.append(&value_entry);
    conv_row.append(&convert_button);

    let conv_dec = Label::builder().label("DEC: -").xalign(0.0).build();
    let conv_hex = Label::builder().label("HEX: -").xalign(0.0).build();
    let conv_bin = Label::builder().label("BIN: -").xalign(0.0).build();
    let conv_error = Label::builder().label("").xalign(0.0).build();

    content.append(&calc_title);
    content.append(&calc_box);
    content.append(&calc_result);
    content.append(&gtk4::Separator::new(Orientation::Horizontal));
    content.append(&conv_title);
    content.append(&conv_row);
    content.append(&conv_dec);
    content.append(&conv_hex);
    content.append(&conv_bin);
    content.append(&conv_error);

    let actions = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .halign(gtk4::Align::End)
        .build();
    let close_button = Button::builder().label("Fermer").build();
    actions.append(&close_button);
    content.append(&actions);

    {
        let calc_entry = calc_entry;
        let calc_result = calc_result;
        calc_button.connect_clicked(move |_| {
            let expression = calc_entry.text().trim().to_string();
            if expression.is_empty() {
                calc_result.set_label("Résultat: expression vide");
                return;
            }

            match meval::eval_str(&expression) {
                Ok(value) => calc_result.set_label(&format!("Résultat: {value}")),
                Err(e) => calc_result.set_label(&format!("Résultat: erreur ({e})")),
            }
        });
    }

    {
        let value_entry = value_entry;
        let base_dropdown = base_dropdown;
        let conv_dec = conv_dec;
        let conv_hex = conv_hex;
        let conv_bin = conv_bin;
        let conv_error = conv_error;

        convert_button.connect_clicked(move |_| {
            let input = value_entry.text().trim().to_string();
            if input.is_empty() {
                conv_error.set_label("Erreur: valeur vide");
                return;
            }

            let base = match base_dropdown.selected() {
                1 => 16,
                2 => 2,
                _ => 10,
            };

            match parse_signed_radix(&input, base) {
                Ok(value) => {
                    conv_dec.set_label(&format!("DEC: {value}"));
                    conv_hex.set_label(&format!("HEX: {}", format_hex(value)));
                    conv_bin.set_label(&format!("BIN: {}", format_bin(value)));
                    conv_error.set_label("");
                }
                Err(e) => conv_error.set_label(&format!("Erreur: {e}")),
            }
        });
    }

    {
        let dialog = dialog.clone();
        close_button.connect_clicked(move |_| {
            dialog.close();
        });
    }

    dialog.set_child(Some(&content));
    dialog.present();
}

fn parse_signed_radix(input: &str, base: u32) -> anyhow::Result<i128> {
    let raw = input.trim();
    let is_negative = raw.starts_with('-');
    let mut digits = if is_negative { &raw[1..] } else { raw };

    if base == 16 {
        digits = digits.trim_start_matches("0x").trim_start_matches("0X");
    } else if base == 2 {
        digits = digits.trim_start_matches("0b").trim_start_matches("0B");
    }

    let unsigned = i128::from_str_radix(digits, base)
        .with_context(|| format!("valeur invalide pour la base {base}"))?;

    if is_negative {
        Ok(-unsigned)
    } else {
        Ok(unsigned)
    }
}

fn format_hex(value: i128) -> String {
    if value < 0 {
        format!("-0x{:X}", -value)
    } else {
        format!("0x{value:X}")
    }
}

fn format_bin(value: i128) -> String {
    if value < 0 {
        format!("-0b{:b}", -value)
    } else {
        format!("0b{value:b}")
    }
}
