use chrono::NaiveDateTime;
use ethers::types::{U256, U64};
use ricolib::{ddso::vat::Ilk, math::units};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::urn::UrnData;

pub struct RightMainPanel {
    pub market_view: Rect,
    pub ilk_view: Rect,
}

pub struct LeftMainPanel {
    pub base_view: Rect,
    pub urn_view: Option<Vec<Rect>>,
    pub menu_view: Option<Rect>,
}

impl LeftMainPanel {
    pub fn new(base_view: Rect) -> Self {
        Self {
            base_view,
            urn_view: None,
            menu_view: None,
        }
    }

    pub fn set_urn_view(&mut self, urn_view: Vec<Rect>) {
        self.urn_view = Some(urn_view);
    }

    #[allow(dead_code)]
    pub fn set_menu_view(&mut self, menu_view: Rect) {
        self.menu_view = Some(menu_view);
    }
}

pub struct TermCanvas {
    pub size: Rect,
    pub navbar: Rect,
    pub left_main_panel: LeftMainPanel,
    pub right_main_pane: RightMainPanel,
    pub footer: Rect,
}

impl TermCanvas {
    pub fn init(size: Rect) -> Self {
        let _areas = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(85), Constraint::Percentage(15)])
            .split(size);

        let _main_views = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(_areas[0]);
        let _left_views = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(_main_views[0]);
        let navbar = _left_views[0];
        let left_main_panel = LeftMainPanel::new(_left_views[1]);
        let _right_views = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(_main_views[1]);
        let right_main_pane = RightMainPanel {
            market_view: _right_views[0],
            ilk_view: _right_views[1],
        };
        let footer = _areas[1];
        Self {
            size,
            navbar,
            left_main_panel,
            right_main_pane,
            footer,
        }
    }
}

pub fn paint_urn(urn: &UrnData, is_active: bool) -> Paragraph {
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
        "art:   {}\ndebt:  {}\n\tink:   {} \nloan/value: {:.12} / {:.12} --> safety: {:.5}",
        urn.art.low_u64() as f64 / 10_u64.pow(18) as f64,
        (urn.debt.as_u128() as f64 / 10_u64.pow(18) as f64),
        ink,
        (urn.loan.as_u128() as f64 / 10_u64.pow(18) as f64),
        (urn.value.as_u128() as f64 / 10_u64.pow(18) as f64),
        urn.safety
    );
    let border_stylestyle = match is_active {
        true => Style::default().fg(Color::White),
        false => Style::default().fg(Color::DarkGray),
    };
    Paragraph::new(urn_text).block(
        Block::default()
            .title(urn.ink_name.as_str())
            .borders(Borders::ALL)
            .border_style(border_stylestyle),
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
            Span::raw(format!("{} UTC ({} seconds ago)", last_refreshed, (chrono::Utc::now().naive_utc() - last_refreshed).num_seconds())),

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

pub fn paint_marpar(
    mar: U256,
    par: U256,
    way: U256,
    tau: U256,
    how: U256,
    current_time: NaiveDateTime,
) -> Paragraph<'static> {
    let units = units::new();
    let price_rate = ((way.as_u128() as f64 / units.RAY_F64).powf(units.BANKYEAR) - 1.0) * 100.0;
    let time_delta =
        current_time - NaiveDateTime::from_timestamp_opt(tau.as_u128() as i64, 0).unwrap();
    let next_way = (0..time_delta.num_seconds()).fold(way, |acc, _| {
        acc * {
            match mar.cmp(&par) {
                std::cmp::Ordering::Less => how,
                std::cmp::Ordering::Greater => units.RAY * units.RAY / how,
                std::cmp::Ordering::Equal => U256::from(0),
            }
        } / units.RAY
    });
    let next_price_rate =
        ((next_way.as_u128() as f64 / units.RAY_F64).powf(units.BANKYEAR) - 1.0) * 100.0;
    let written_text = match mar.cmp(&par) {
        std::cmp::Ordering::Greater => format!(
            "mar > par, price rate is decreasing (currently {:.6}%)",
            price_rate
        ),
        std::cmp::Ordering::Less => format!(
            "mar < par, price rate is increasing (currently {:.6}%)",
            price_rate
        ),
        std::cmp::Ordering::Equal => format!(
            "mar = par, price rate is stable (currently {:.6}%)",
            price_rate
        ),
    };

    #[allow(clippy::format_in_format_args)]
    let marpar_text = format!(
        "par: {}\nmar: {}\nmsg: {}\nlast poke: {} (would be {:.6}%)",
        par.as_u128() as f64 / 10_u128.pow(27) as f64,
        mar.as_u128() as f64 / 10_u128.pow(27) as f64,
        written_text,
        format!(
            "{} hours, {} minutes, {} seconds ago",
            time_delta.num_seconds() / 3600,
            (time_delta.num_seconds() % 3600) / 60,
            time_delta.num_seconds() % 60
        ),
        next_price_rate
    );
    Paragraph::new(marpar_text).block(
        Block::default()
            .title("mar/par")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    ) // This sets the border color to red)
}
// define the paint_menu function. It should take in a Vec<String> and return a Paragraph where each string is on a new line with numbering.
// The first argument is the menu items, and the second argument is the selected index. The selected index should be highlighted.

pub fn paint_menu(menu_items: Vec<&str>, selected_index: usize) -> Paragraph {
    let mut menu_spans = vec![];
    for (i, item) in menu_items.iter().enumerate() {
        if i == selected_index {
            menu_spans.push(Spans::from(vec![Span::styled(
                format!("{}. {}\n", i + 1, item),
                Style::default().add_modifier(Modifier::REVERSED),
            )]));
        } else {
            menu_spans.push(Spans::from(vec![Span::raw(format!(
                "{}. {}\n",
                i + 1,
                item
            ))]));
        }
    }
    let menu_text = Text::from(menu_spans);
    Paragraph::new(menu_text).block(
        Block::default()
            .title("Menu")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White)),
    )
}

pub fn paint_pricing_screen(mar: U256, par: U256, xau: U256) -> Paragraph<'static> {
    let units = units::new();
    let mar_usd =
        ((mar * xau * units.BLN) / units.RAY / units.RAY).as_u128() as f64 / units.BLN_F64;
    let par_usd =
        ((par * xau * units.BLN) / units.RAY / units.RAY).as_u128() as f64 / units.BLN_F64;
    let xau_usd = (xau * units.BLN / units.RAY).as_u128() as f64 / units.BLN_F64;

    let pricing_text = format!(
        "mar: ~{:.6} USD\npar: ~{:.6} USD\nxau: ~{:.6} USD",
        mar_usd, par_usd, xau_usd
    );
    Paragraph::new(pricing_text).block(
        Block::default()
            .title("$$$")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    )
}
