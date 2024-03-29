use std::collections::HashMap;

use chrono::NaiveDateTime;
use ethers::types::{H160, U256, U64};
use ricolib::{
    ddso::{
        events::{NewPalm0, NewPalm2, Palms},
        vat::Ilk,
    },
    math::units,
    utils::bytes32_to_string,
};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
};

use crate::{config::TermConfig, urn::UrnData, ChainData, SelectedActiveView, State};

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
    pub color_map: std::collections::HashMap<&'static str, Color>,
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

        let color_map = HashMap::from([
            ("weth", Color::Green),
            ("usdc", Color::Blue),
            (":uninft", Color::Magenta),
            ("way", Color::Rgb(51, 204, 197)),
            ("tau", Color::Rgb(245, 158, 66)),
            ("par", Color::Rgb(29, 219, 156)),
        ]);
        Self {
            size,
            navbar,
            left_main_panel,
            right_main_pane,
            footer,
            color_map,
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
        None => ethers::utils::format_units(
            urn.ink,
            match urn.ink_name.as_str() {
                "usdc" => 6,
                _ => 18,
            },
        )
        .unwrap(),
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
        "  tart: {}\n  tink: {}\n  rack: {}\n  rho: {} UTC ({} hours ago)\n fee: {}%",
        (((ilk.tart * units.BLN) / units.WAD).as_u128() as f64 / units.BLN_F64),
        ethers::utils::format_units(
            ilk.tink.unwrap_or(U256::zero()),
            ilk.inkd.unwrap_or(U256::zero()).as_u64() as u32
        )
        .unwrap(),
        (((ilk.rack * units.BLN) / units.RAY).as_u128() as f64 / units.BLN_F64),
        NaiveDateTime::from_timestamp_opt(ilk.rho.as_u128() as i64, 0).unwrap(),
        time_since_update_string,
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

pub fn paint_newpalm2s<'a>(
    palms: Vec<&NewPalm2>,
    color_map: &'a std::collections::HashMap<&'a str, Color>,
    last_block: U64,
) -> Paragraph<'a> {
    // assume already filtered
    let units = units::new();
    let header = Spans::from(vec![Span::styled(
        format!("{} ...", last_block),
        Style::default()
            .add_modifier(Modifier::ITALIC)
            .add_modifier(Modifier::SLOW_BLINK),
    )]);

    let text = palms
        .iter()
        .map(|log| {
            Spans::from(vec![
                Span::raw(format!(
                    "{}  {}  ",
                    log.block_number,
                    bytes32_to_string(log.act)
                )),
                Span::styled(
                    bytes32_to_string(log.ilk),
                    Style::default().fg(color_map
                        .get(bytes32_to_string(log.ilk).as_str())
                        .unwrap_or(&Color::Reset)
                        .to_owned()),
                ), // Styled part
                Span::raw(format!(
                    "  {}  {:.6}\n",
                    H160::from_slice(&log.usr.as_bytes()[..20]),
                    log.val.as_u128() as f64 / units.WAD_F64
                )),
            ])
        })
        .collect::<Vec<Spans>>();
    // prepend header
    let text = vec![header].into_iter().chain(text).collect::<Vec<Spans>>();
    Paragraph::new(text)
}

pub fn paint_newpalm0s<'a>(
    palms: Vec<&NewPalm0>,
    color_map: &'a std::collections::HashMap<&'a str, Color>,
    last_block: U64,
) -> Paragraph<'a> {
    let header = Spans::from(vec![Span::styled(
        format!("{} ...", last_block),
        Style::default()
            .add_modifier(Modifier::ITALIC)
            .add_modifier(Modifier::SLOW_BLINK),
    )]);

    let text = palms
        .iter()
        .map(|log| {
            Spans::from(vec![
                Span::raw(format!("{}    ", log.block_number,)),
                Span::styled(
                    format!(
                        "{}    {}",
                        bytes32_to_string(log.which),
                        match bytes32_to_string(log.which).as_str() {
                            "way" => {
                                let units = units::new();
                                format!(
                                    "{:.6}%",
                                    ((U256::from_big_endian(log.what.as_bytes()).as_u128() as f64
                                        / units.RAY_F64)
                                        .powf(units.BANKYEAR)
                                        - 1.0)
                                        * 100.0
                                )
                            }
                            "tau" => {
                                // Need to convert to H256 to epoch seconds and then represent as date string
                                let tau256 = U256::from_big_endian(log.what.as_bytes());
                                let time = chrono::NaiveDateTime::from_timestamp_opt(
                                    tau256.as_u128() as i64,
                                    0,
                                )
                                .unwrap();
                                // Format to only show month/day hour/minute. E.g. 12/31 23:59
                                time.format("%H:%M %b %d").to_string()
                            }
                            "par" => {
                                format!(
                                    "{}",
                                    U256::from_big_endian(log.what.as_bytes()).as_u128() as f64
                                        / 10_u128.pow(27) as f64
                                )
                            }
                            "debt" => {
                                format!(
                                    "{:.6}",
                                    U256::from_big_endian(log.what.as_bytes()).as_u128() as f64
                                        / 10_u128.pow(18) as f64
                                )
                            }
                            "ceil" => {
                                format!(
                                    "{:.6}",
                                    U256::from_big_endian(log.what.as_bytes()).as_u128() as f64
                                        / 10_u128.pow(18) as f64
                                )
                            }
                            "joy" => {
                                format!(
                                    "{:.6}",
                                    U256::from_big_endian(log.what.as_bytes()).as_u128() as f64
                                        / 10_u128.pow(18) as f64
                                )
                            }
                            _ => log.what.to_string(),
                        }
                    ),
                    Style::default().fg(color_map
                        .get(bytes32_to_string(log.which).as_str())
                        .unwrap_or(&Color::Reset)
                        .to_owned()),
                ),
            ])
        })
        .collect::<Vec<Spans>>();
    // prepend header
    let text = vec![header].into_iter().chain(text).collect::<Vec<Spans>>();
    Paragraph::new(text)
}

pub fn paint_settings(config: &crate::config::TermConfig) -> Paragraph {
    let text = format!(
        "settings:\nrpc_url: {}\n refresh_freq: {} seconds \nwallet_address: {}\nilks: {}",
        config.rpc.arb_rpc_url,
        config.rpc.refresh_seconds,
        config.urns.user_address,
        config.urns.ilks.join(", ")
    );
    Paragraph::new(text)
}

pub fn paint_active_view<'a>(
    state: &State,
    data: &'a ChainData,
    color_map: &'a std::collections::HashMap<&'a str, Color>,
    config: &'a TermConfig,
) -> (Paragraph<'a>, &'a str) {
    let (active_text, active_title) = match &state.selected_active_view {
        SelectedActiveView::Settings => (paint_settings(config), "settings"),
        SelectedActiveView::Ilk => {
            if !state.active_ilk.is_empty() {
                let all_ilk_data = &data.ilks;
                // now, instead, iterate over all active ilks and append their data to the active view
                let formatted_str = state
                    .active_ilk
                    .iter()
                    .enumerate()
                    .map(|(index, ilk)| {
                        format!(
                            "ilk: {}\n{}",
                            ilk,
                            match all_ilk_data.get(index) {
                                Some(ilk) => {
                                    paint_ilk(ilk, data.last_refreshed)
                                }
                                _ => "  Awaiting ilk data...".to_string(),
                            }
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");
                (Paragraph::new(formatted_str), "ilk_view")
            } else {
                (Paragraph::new("No active view".to_string()), "active_view")
            }
        }
        SelectedActiveView::NewPalm2 => {
            if !data.logs.is_empty() {
                let filtered_logs = data
                    .logs
                    .iter() // also filter on that Palm type is NewPalm2
                    .filter_map(|log| {
                        if let Palms::NewPalm2(palm) = log {
                            if state.active_ilk.contains(&bytes32_to_string(palm.ilk))
                                || state.active_ilk.is_empty()
                            {
                                Some(palm)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<&NewPalm2>>();
                let logs_text = paint_newpalm2s(filtered_logs, color_map, data.block);
                (logs_text, "frob/bail")
            } else {
                (Paragraph::new("Awaiting NewPalm2 event..."), "frob/bail")
            }
        }
        SelectedActiveView::NewPalm0 => {
            if !data.logs.is_empty() {
                let filtered_logs = data
                    .logs
                    .iter() // also filter on that Palm type is NewPalm0
                    .filter_map(|log| {
                        if let Palms::NewPalm0(palm) = log {
                            Some(palm)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<&NewPalm0>>();
                let logs_text = paint_newpalm0s(filtered_logs, color_map, data.block);
                (logs_text, "sys-events")
            } else {
                (Paragraph::new("Awaiting NewPalm0 event..."), "sys-events")
            }
        }
        _ => (Paragraph::new("No active view"), "active_view"),
    };

    (active_text, active_title)
}
