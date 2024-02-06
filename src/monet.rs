use chrono::NaiveDateTime;
use ethers::types::{U256, U64};
use ricolib::{ddso::vat::Ilk, math::units};
use tui::{
    style::{Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::urn::UrnData;

pub fn paint_urn(urn: &UrnData) -> Paragraph {
    let ink = match &urn.ninks {
        Some(ninks) => ninks
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(", "),
        None => format!(
            "{}",
            urn.ink.low_u64() as f64
                / match urn.ink_name.as_str() {
                    "usdc" => 10_u64.pow(6) as f64,
                    _ => 10_u64.pow(18) as f64,
                }
        ),
    };
    let urn_text = format!(
        "art: {}\n\tink: {} \nloan: {} \nvalue: {}\nsafety: {}",
        urn.art.low_u64() as f64 / 10_u64.pow(18) as f64,
        ink,
        (urn.loan.as_u128() as f64 / 10_u64.pow(18) as f64),
        (urn.value.as_u128() as f64 / 10_u64.pow(18) as f64),
        urn.safety
    );
    Paragraph::new(urn_text).block(
        Block::default()
            .title(urn.ink_name.as_str())
            .borders(Borders::ALL),
    )
}

pub fn paint_ilk<T: From<String>>(ilk: &Ilk, current_time: NaiveDateTime) -> T {
    let units = units::new();
    let time_since_update =
        current_time - NaiveDateTime::from_timestamp_opt(ilk.rho.as_u128() as i64, 0).unwrap();
    let time_since_update_string = format!(
        "{} hours, {} minutes, {} seconds",
        time_since_update.num_seconds() / 3600,
        (time_since_update.num_seconds() % 3600) / 60,
        time_since_update.num_seconds() % 60
    );
    format!(
        "  tart: {}\n  rack: {}\n  rho: {} UTC ({} hours ago)\n  dust: {}\n  fee: {}%",
        (((ilk.tart * units.BLN) / units.WAD).as_u128() as f64 / units.BLN_F64),
        (((ilk.rack * units.BLN) / units.RAY).as_u128() as f64 / units.BLN_F64),
        NaiveDateTime::from_timestamp_opt(ilk.rho.as_u128() as i64, 0).unwrap(),
        time_since_update_string,
        (((ilk.dust * units.BLN) / units.RAD).as_u128() as f64 / units.BLN_F64),
        (((ilk.fee * units.WAD) / units.RAY).as_u128() as f64 / units.BLN_F64.powf(2.0) - 1_f64)
            * units.BANKYEAR
            * 100.0
    )
    .into()
}

pub fn paint_footer(
    last_block: U64,
    last_refreshed: NaiveDateTime,
    ilk_shortcuts: &str,
) -> Paragraph {
    let footer_spans = vec![
        Spans::from(vec![
            Span::styled("Last Block: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} ", last_block)),
            Span::styled("Last Refreshed: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} UTC ", last_refreshed)),
        ]),
        Spans::from(vec![
            Span::styled("global_controls: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("'q' to quit, 'p' to pop last ilk, 'c' to clear active view, 's' to view settings\n"),
        ]),
        Spans::from(vec![
            Span::styled("ilk_shortcuts: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(ilk_shortcuts),
        ]),
    ];
    // Converting the collection of Spans into a single Text object
    let footer_text = Text::from(footer_spans);

    Paragraph::new(footer_text).block(
        Block::default()
            .title("ricoterm info")
            .borders(Borders::ALL),
    )
}

pub fn paint_marpar(mar: U256, par: U256) -> Paragraph<'static> {
    let written_text = match mar.cmp(&par) {
        std::cmp::Ordering::Greater => "mar > par, price rate is decreasing",
        std::cmp::Ordering::Less => "mar < par, price rate is increasing",
        std::cmp::Ordering::Equal => "mar = par",
    };

    let marpar_text = format!(
        "par: {}\nmar: {}\n{}",
        par.as_u128() as f64 / 10_u128.pow(27) as f64,
        mar.as_u128() as f64 / 10_u128.pow(27) as f64,
        written_text
    );
    Paragraph::new(marpar_text).block(Block::default().title("mar/par").borders(Borders::ALL))
}
