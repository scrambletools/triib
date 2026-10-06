//! The entity list: a row for each entity, the name first, then the
//! columns the user picked. Each column's heading is a menu that removes
//! the column or shows another field in it, and a last heading adds one.

use std::fmt;

use atdecc::DiscoveredEntity;
use atdecc::descriptor::DescriptorType;
use atdecc::model::EntityModel;
use iced::widget::text::Wrapping;
use iced::widget::{container, pick_list, scrollable, table};
use iced::{Background, Border, Center, Color, Element, Fill, Length, Theme};
use scramble_ui::button::{self, Kind};
use scramble_ui::component;
use scramble_ui::font::{TEXT, Type, styled};
use scramble_ui::icon;
use scramble_ui::{Scheme, shape, style};

use crate::app::{Message, Triib};
use crate::column_resize::{MIN_WIDTH, resizable};
use crate::describe;
use crate::settings::EntityField;
use crate::text::measure;
use crate::view::Rate;

/// Around each cell, around the table, and spare room for what measuring
/// its text leaves out.
const TABLE_PADDING: f32 = 12.0;
const TABLE_MARGIN: f32 = 16.0;
const TABLE_SLACK: f32 = 24.0;
/// What a heading's menu adds to its label: padding and the arrow.
const MENU_ROOM: f32 = 36.0;
/// The width of the heading that adds a column.
const ADD_WIDTH: f32 = 40.0;
const MEDIUM: iced::Font = iced::Font {
    weight: iced::font::Weight::Medium,
    ..TEXT
};

pub fn view(triib: &Triib) -> Element<'_, Message> {
    iced::widget::responsive(move |size| view_at(triib, size.width)).into()
}

/// What a column's value is for an entity.
fn value(field: EntityField, entity: &DiscoveredEntity, model: Option<&EntityModel>) -> String {
    let descriptor = model.and_then(EntityModel::entity);
    let localized = |reference| {
        model
            .and_then(|model| model.localized(reference))
            .unwrap_or_default()
            .to_owned()
    };
    match field {
        EntityField::Group => descriptor.map_or_else(String::new, |found| found.group_name.into()),
        EntityField::Product => model.map_or_else(String::new, describe::product),
        EntityField::Vendor => {
            descriptor.map_or_else(String::new, |found| localized(found.vendor_name))
        }
        EntityField::Model => {
            descriptor.map_or_else(String::new, |found| localized(found.model_name))
        }
        EntityField::Firmware => {
            descriptor.map_or_else(String::new, |found| found.firmware_version.into())
        }
        EntityField::SerialNumber => {
            descriptor.map_or_else(String::new, |found| found.serial_number.into())
        }
        EntityField::Milan => model.map_or_else(String::new, describe::milan),
        EntityField::Roles => describe::roles(&entity.adp),
        EntityField::MediaClock => model.map_or_else(String::new, describe::media_clock),
        EntityField::SamplingRate => model
            .and_then(|model| model.audio_units().next())
            .map_or_else(String::new, |unit| {
                Rate(unit.current_sampling_rate).to_string()
            }),
        EntityField::ClockSource => model
            .and_then(|model| {
                let domain = model.clock_domains().next()?;
                model.name_of(DescriptorType::CLOCK_SOURCE, domain.clock_source_index)
            })
            .unwrap_or_default()
            .to_owned(),
        EntityField::Btc => describe::clock(&entity.adp),
        EntityField::State => describe::read_state(model),
        EntityField::EntityId => entity.entity_id().to_string(),
        EntityField::MacAddress => entity.mac.to_string(),
        EntityField::EntityModelId => entity.adp.entity_model_id.to_string(),
        EntityField::Configuration => model
            .and_then(EntityModel::configuration)
            .map(|configuration| configuration.object_name)
            .filter(|name| !name.is_empty())
            .map_or_else(
                || (entity.adp.current_configuration_index + 1).to_string(),
                str::to_owned,
            ),
        EntityField::TalkerStreams => count(entity.adp.talker_stream_sources),
        EntityField::ListenerStreams => count(entity.adp.listener_stream_sinks),
    }
}

/// A count of streams, or nothing for none.
fn count(streams: u16) -> String {
    if streams == 0 {
        String::new()
    } else {
        streams.to_string()
    }
}

/// A column of the entity list: the name, or a field after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Field(EntityField),
}

/// What a heading's menu offers.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Choice {
    Remove,
    MoveLeft,
    MoveRight,
    Field(EntityField),
}

impl fmt::Display for Choice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Choice::Remove => formatter.write_str("Remove column"),
            Choice::MoveLeft => formatter.write_str("Move left"),
            Choice::MoveRight => formatter.write_str("Move right"),
            Choice::Field(field) => formatter.write_str(field.label()),
        }
    }
}

/// The fields no column shows, in the menus' order.
fn unshown(columns: &[EntityField]) -> impl Iterator<Item = EntityField> + '_ {
    EntityField::ALL
        .into_iter()
        .filter(|field| !columns.contains(field))
}

/// A heading as a menu: quiet like the other headings until pointed at.
fn heading_menu(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let scheme = Scheme::of(theme);
    let border = match status {
        pick_list::Status::Active => Color::TRANSPARENT,
        pick_list::Status::Hovered => scheme.outline_variant,
        pick_list::Status::Opened { .. } => scheme.primary,
    };
    pick_list::Style {
        text_color: scheme.on_surface_variant,
        placeholder_color: scheme.on_surface_variant,
        handle_color: scheme.on_surface_variant,
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: border,
            width: 1.0,
            radius: shape::EXTRA_SMALL.into(),
        },
    }
}

/// The table for `width` pixels: across the whole width when its columns
/// fit, each widened in proportion to its text, else at their own widths,
/// scrolling sideways.
fn view_at(triib: &Triib, width: f32) -> Element<'_, Message> {
    #[derive(Clone)]
    struct Row<'a> {
        entity: &'a DiscoveredEntity,
        name: String,
        values: Vec<String>,
    }
    let columns: &[EntityField] = &triib.settings.entity_columns;
    let rows: Vec<Row<'_>> = triib
        .shown_entities()
        .into_iter()
        .map(|entity| {
            let model = triib.models.get(&entity.entity_id());
            Row {
                entity,
                name: triib.entity_name(entity),
                values: columns
                    .iter()
                    .map(|&field| value(field, entity, model))
                    .collect(),
            }
        })
        .collect();
    let can_add = unshown(columns).next().is_some();
    // Each column's own width: its widest text, its heading with the menu's
    // arrow, and for the name its icon and the row button's padding.
    let widest = |heading: f32, texts: &mut dyn Iterator<Item = &String>| {
        texts
            .map(|text| measure(text, TEXT, 14.0))
            .fold(heading, f32::max)
    };
    let mut natural = vec![
        widest(
            measure("Name", MEDIUM, 14.0),
            &mut rows.iter().map(|row| &row.name),
        ) + 20.0
            + 8.0
            + 32.0,
    ];
    for (index, field) in columns.iter().enumerate() {
        natural.push(widest(
            measure(field.label(), MEDIUM, 14.0) + MENU_ROOM,
            &mut rows.iter().map(|row| &row.values[index]),
        ));
    }
    // The widths the user dragged columns to, which they keep.
    let settings = &triib.settings;
    let mut dragged: Vec<Option<f32>> = vec![settings.entity_name_width];
    dragged.extend(
        columns
            .iter()
            .map(|field| settings.entity_column_widths.get(field).copied()),
    );
    if can_add {
        natural.push(ADD_WIDTH);
        dragged.push(None);
    }
    let needed = natural
        .iter()
        .zip(&dragged)
        .map(|(natural, dragged)| dragged.unwrap_or(*natural))
        .sum::<f32>()
        + natural.len() as f32 * (2.0 * TABLE_PADDING + 1.0)
        + 2.0 * TABLE_MARGIN
        + TABLE_SLACK;
    let fits = width >= needed;
    let column_width = |column: usize| match dragged[column] {
        Some(dragged) => Length::Fixed(dragged.max(MIN_WIDTH)),
        None if fits => Length::FillPortion(natural[column].ceil().max(1.0) as u16),
        None => Length::Fixed(natural[column].ceil()),
    };
    // Headings and cells as wide as their columns: a table lays out a cell
    // that fills apart from its column's width.
    let heading = |content: Element<'static, Message>, column: Column, width: Length| {
        resizable(
            content,
            move |width| Message::EntityColumnResized(column, width),
            Message::EntityColumnResizeEnded,
            Message::EntityColumnWidthReset(column),
        )
        .width(width)
    };
    // A cell's text on one line, cut off at the column's edge.
    let cell = |text: String, width: Length| {
        container(styled(text, Type::BodyMedium).wrapping(Wrapping::None))
            .width(width)
            .clip(true)
    };
    let name_width = column_width(0);

    let mut table_columns = vec![
        table::column(
            heading(
                styled("Name", Type::LabelLarge)
                    .style(style::on_surface_variant)
                    .into(),
                Column::Name,
                name_width,
            ),
            move |row: Row<'_>| {
                let entity_id = row.entity.entity_id();
                container(
                    button::custom(
                        Kind::Row,
                        iced::widget::row![
                            icon::icon(describe::glyph(&row.entity.adp), 20),
                            styled(row.name, Type::BodyMedium).wrapping(Wrapping::None),
                        ]
                        .spacing(8)
                        .align_y(Center),
                    )
                    .selected(triib.selected == Some(entity_id))
                    .on_press(Message::EntitySelected(entity_id)),
                )
                .width(name_width)
                .clip(true)
            },
        )
        .width(column_width(0))
        .align_y(Center),
    ];
    let last = columns.len().saturating_sub(1);
    for (index, &field) in columns.iter().enumerate() {
        let mut choices = vec![Choice::Remove];
        if index > 0 {
            choices.push(Choice::MoveLeft);
        }
        if index < last {
            choices.push(Choice::MoveRight);
        }
        choices.extend(unshown(columns).map(Choice::Field));
        let menu = pick_list(
            choices,
            Some(Choice::Field(field)),
            move |choice| match choice {
                Choice::Remove => Message::EntityColumn(index, None),
                Choice::MoveLeft => Message::EntityColumnMoved(index, false),
                Choice::MoveRight => Message::EntityColumnMoved(index, true),
                Choice::Field(field) => Message::EntityColumn(index, Some(field)),
            },
        )
        .font(MEDIUM)
        .text_size(14)
        .padding([2, 6])
        .style(heading_menu)
        .menu_style(style::select_menu);
        let width = column_width(index + 1);
        table_columns.push(
            table::column(
                heading(menu.into(), Column::Field(field), width),
                move |row: Row<'_>| cell(row.values[index].clone(), width),
            )
            .width(width)
            .align_y(Center),
        );
    }
    if can_add {
        let add = pick_list(
            unshown(columns).map(Choice::Field).collect::<Vec<_>>(),
            None::<Choice>,
            |choice| match choice {
                Choice::Field(field) => Message::EntityColumnAdded(field),
                _ => Message::Nothing,
            },
        )
        .placeholder("+")
        .handle(pick_list::Handle::None)
        .font(MEDIUM)
        .text_size(16)
        .padding([2, 6])
        .style(heading_menu)
        .menu_style(style::select_menu);
        table_columns.push(
            table::column(component::tip(add, "Add a column"), |_row: Row<'_>| {
                iced::widget::space()
            })
            .width(column_width(columns.len() + 1))
            .align_y(Center),
        );
    }
    let mut table = table(table_columns, rows)
        .padding_x(TABLE_PADDING)
        .padding_y(8);
    if fits {
        table = table.width(Fill);
    }
    let content = container(table).padding(TABLE_MARGIN);
    if fits {
        return component::scroll(content).height(Fill).into();
    }
    scrollable(content)
        .direction(scrollable::Direction::Both {
            vertical: component::thin_scrollbar(),
            horizontal: component::thin_scrollbar(),
        })
        .style(style::scrollbar)
        .width(Fill)
        .height(Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_change_go_and_come() {
        use crate::settings::Settings;

        let (entities, models) = crate::view::tests::bench();
        let interface = avb_net::Interface {
            name: "enp6s0".to_owned(),
            mac: avb_net::MacAddress([0x9c, 0x6b, 0x00, 0x30, 0x9a, 0x2b]),
            up: true,
            speed: None,
            physical: true,
            wireless: false,
            hardware_clock: None,
        };
        let mut triib = Triib::sample(Settings::default(), interface, entities, models);
        let columns = |triib: &Triib| triib.settings.entity_columns.clone();
        // Group becomes Vendor; a field shown elsewhere is not taken twice.
        let _ = triib.update(Message::EntityColumn(0, Some(EntityField::Vendor)));
        assert_eq!(columns(&triib)[0], EntityField::Vendor);
        let _ = triib.update(Message::EntityColumn(0, Some(EntityField::Btc)));
        assert_eq!(columns(&triib)[0], EntityField::Vendor);
        // Product goes; MAC address comes at the right.
        let _ = triib.update(Message::EntityColumn(1, None));
        assert!(!columns(&triib).contains(&EntityField::Product));
        let _ = triib.update(Message::EntityColumnAdded(EntityField::MacAddress));
        assert_eq!(columns(&triib).last(), Some(&EntityField::MacAddress));
        assert_eq!(columns(&triib).len(), EntityField::DEFAULT_COLUMNS.len());
        // A place past the last column changes nothing.
        let before = columns(&triib);
        let _ = triib.update(Message::EntityColumn(99, None));
        assert_eq!(columns(&triib), before);

        // The columns survive saving and loading.
        let file = std::env::temp_dir().join(format!("triib-columns-{}.toml", std::process::id()));
        triib_store::save(&file, &triib.settings).unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        let loaded: Settings = triib_store::load_or_create(&file).unwrap();
        let _ = std::fs::remove_file(&file);
        assert!(saved.contains("\"mac-address\""), "{saved}");
        assert_eq!(loaded.entity_columns, before);
    }

    fn sample() -> Triib {
        let (entities, models) = crate::view::tests::bench();
        let interface = avb_net::Interface {
            name: "enp6s0".to_owned(),
            mac: avb_net::MacAddress([0x9c, 0x6b, 0x00, 0x30, 0x9a, 0x2b]),
            up: true,
            speed: None,
            physical: true,
            wireless: false,
            hardware_clock: None,
        };
        let settings = crate::settings::Settings {
            view: crate::settings::View::Entities,
            ..crate::settings::Settings::default()
        };
        Triib::sample(settings, interface, entities, models)
    }

    #[test]
    fn columns_move_and_keep_their_widths() {
        let mut triib = sample();
        let columns = |triib: &Triib| triib.settings.entity_columns.clone();
        // Group moves right past Product, then back; the ends stay put.
        let _ = triib.update(Message::EntityColumnMoved(0, true));
        assert_eq!(
            &columns(&triib)[..2],
            [EntityField::Product, EntityField::Group]
        );
        let _ = triib.update(Message::EntityColumnMoved(1, false));
        assert_eq!(
            &columns(&triib)[..2],
            [EntityField::Group, EntityField::Product]
        );
        let before = columns(&triib);
        let _ = triib.update(Message::EntityColumnMoved(0, false));
        let _ = triib.update(Message::EntityColumnMoved(before.len() - 1, true));
        assert_eq!(columns(&triib), before);

        // Dragged widths stay, by field, until reset.
        let _ = triib.update(Message::EntityColumnResized(Column::Name, 240.0));
        let _ = triib.update(Message::EntityColumnResized(
            Column::Field(EntityField::Firmware),
            90.0,
        ));
        let _ = triib.update(Message::EntityColumnResizeEnded);
        assert_eq!(triib.settings.entity_name_width, Some(240.0));
        assert_eq!(
            triib
                .settings
                .entity_column_widths
                .get(&EntityField::Firmware),
            Some(&90.0)
        );

        // Fields, order and widths survive saving and loading.
        let file = std::env::temp_dir().join(format!("triib-widths-{}.toml", std::process::id()));
        triib_store::save(&file, &triib.settings).unwrap();
        let loaded: crate::settings::Settings = triib_store::load_or_create(&file).unwrap();
        let _ = std::fs::remove_file(&file);
        assert_eq!(loaded.entity_columns, triib.settings.entity_columns);
        assert_eq!(
            loaded.entity_column_widths,
            triib.settings.entity_column_widths
        );
        assert_eq!(loaded.entity_name_width, Some(240.0));

        let _ = triib.update(Message::EntityColumnWidthReset(Column::Field(
            EntityField::Firmware,
        )));
        let _ = triib.update(Message::EntityColumnWidthReset(Column::Name));
        assert!(triib.settings.entity_column_widths.is_empty());
        assert_eq!(triib.settings.entity_name_width, None);
    }

    #[test]
    fn dragging_a_headings_edge_resizes_its_column() {
        use iced::mouse::{Button, Event as Mouse};
        use iced::{Event, Point, Size};

        let triib = sample();
        let settings = iced::Settings {
            fonts: scramble_ui::font::files().collect(),
            default_font: TEXT,
            ..iced::Settings::default()
        };
        let mut simulator = iced_test::Simulator::with_size(
            settings,
            Size::new(2200.0, 420.0),
            crate::view::window(&triib),
        );
        // The Group heading's right edge, then 50 pixels further right.
        let edge = Point::new(436.0, 92.0);
        let moved = Point::new(486.0, 92.0);
        simulator.point_at(edge);
        let _ = simulator.simulate([Event::Mouse(Mouse::ButtonPressed(Button::Left))]);
        simulator.point_at(moved);
        let _ = simulator.simulate([Event::Mouse(Mouse::CursorMoved { position: moved })]);
        let _ = simulator.simulate([Event::Mouse(Mouse::ButtonReleased(Button::Left))]);
        let messages: Vec<Message> = simulator.into_messages().collect();
        let widths: Vec<f32> = messages
            .iter()
            .filter_map(|message| match message {
                Message::EntityColumnResized(Column::Field(EntityField::Group), width) => {
                    Some(*width)
                }
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 1, "{messages:?}");
        // The heading was about 117 pixels wide inside its padding.
        assert!((150.0..190.0).contains(&widths[0]), "{widths:?}");
        assert!(
            messages
                .iter()
                .any(|message| matches!(message, Message::EntityColumnResizeEnded))
        );

        // A double click on the edge asks for the column to fit its text.
        let mut simulator = iced_test::Simulator::with_size(
            iced::Settings {
                fonts: scramble_ui::font::files().collect(),
                default_font: TEXT,
                ..iced::Settings::default()
            },
            Size::new(2200.0, 420.0),
            crate::view::window(&triib),
        );
        simulator.point_at(edge);
        for _ in 0..2 {
            let _ = simulator.simulate([
                Event::Mouse(Mouse::ButtonPressed(Button::Left)),
                Event::Mouse(Mouse::ButtonReleased(Button::Left)),
            ]);
        }
        assert!(simulator.into_messages().any(|message| matches!(
            message,
            Message::EntityColumnWidthReset(Column::Field(EntityField::Group))
        )));
    }

    #[test]
    fn values_come_from_the_advertisement_and_the_model() {
        let (entities, models) = crate::view::tests::bench();
        let mac_mini = atdecc::EntityId(0xd111_e597_f544_8000);
        let entity = &entities[&mac_mini];
        let model = models.get(&mac_mini);
        assert_eq!(
            value(EntityField::MacAddress, entity, model),
            entity.mac.to_string()
        );
        assert_eq!(value(EntityField::Vendor, entity, model), "Apple Inc.");
        assert_eq!(
            value(EntityField::ClockSource, entity, model),
            "Mac System Clock"
        );
        assert!(!value(EntityField::SamplingRate, entity, model).is_empty());
    }

    #[test]
    fn menus_offer_only_what_no_column_shows() {
        let columns = [EntityField::Group, EntityField::Btc];
        let offered: Vec<EntityField> = unshown(&columns).collect();
        assert_eq!(offered.len(), EntityField::ALL.len() - 2);
        assert!(!offered.contains(&EntityField::Group));
        assert!(!offered.contains(&EntityField::Btc));
        assert_eq!(offered[0], EntityField::Product);
        assert_eq!(unshown(&EntityField::ALL).count(), 0);
    }
}
