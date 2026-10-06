//! The application: its state, messages and how they change it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use atdecc::descriptor::DescriptorType;
use atdecc::model::EntityModel;
use atdecc::{DiscoveredEntity, EntityId};
use avb_net::Interface;
use iced::{Color, Element, Subscription, Task, Theme};
use scramble_ui::appearance::{self, Appearance, Inputs};
use scramble_ui::{desktop, motion, omarchy, scheme};

use crate::External;
use crate::controls_view::ControlDrag;
use crate::entity_table::Column;
use crate::matrix::{Hover, Side};
use crate::netmap::{Focus, NodeKey};
use crate::network::{Action, Failure, Name, NameTarget, Neighbor, Network, Report, ReportKind};
use crate::settings::{EntityField, NetworkShows, Settings, Streams, View};
use crate::settings_view::SettingsTab;
use crate::view::InspectorTab;

/// The desktop app ID, also the Wayland app ID.
pub const APP_ID: &str = if triib_store::paths::PRODUCTION {
    "io.github.scrambletools.triib"
} else {
    "io.github.scrambletools.triib.dev"
};

/// triib's own accent, when the desktop gives none.
pub const TRIIB_SEED: Color = Color::from_rgb8(0x00, 0x89, 0x7b);

pub struct Triib {
    pub settings: Settings,
    theme: Theme,
    omarchy_dir: Option<PathBuf>,
    pub omarchy: Option<omarchy::Palette>,
    /// The desktop's accent color, when it has one.
    pub system_accent: Option<Color>,
    system_mode: iced::theme::Mode,
    /// The desktop allows animations.
    pub system_animations: bool,
    pub interfaces: Vec<Interface>,
    pub search: String,
    network: Option<Network>,
    generation: u64,
    pub network_state: NetworkState,
    /// The entities online on the chosen interface.
    pub entities: BTreeMap<EntityId, DiscoveredEntity>,
    /// What has been read of each.
    pub models: BTreeMap<EntityId, EntityModel>,
    pub selected: Option<EntityId>,
    /// Actions sent to the network and not finished yet.
    pub pending: Vec<Action>,
    /// What happened to the last action that did not succeed.
    pub notice: Option<String>,
    /// Talkers and listeners the matrix shows as one column or row.
    pub collapsed: BTreeSet<(Side, EntityId)>,
    /// What the pointer is over in the matrix.
    pub hover: Hover,
    /// The bridge this computer is plugged into.
    pub neighbor: Option<Neighbor>,
    /// What the network view brings forward.
    pub network_focus: Option<Focus>,
    /// The toolbar's "More" menu is open.
    pub overflow_open: bool,
    /// On a narrow window, the network view shows its list instead of the map.
    pub network_list: bool,
    /// The name being edited in the inspector, and its text so far.
    pub editing: Option<(NameTarget, String)>,
    /// The descriptor types opened in the inspector's tree, by entity.
    pub tree_open: BTreeSet<(EntityId, DescriptorType)>,
    /// The control slider being dragged.
    pub control_drag: Option<ControlDrag>,
    /// The inspector's open tab.
    pub inspector_tab: InspectorTab,
    /// The Settings dialog is open, on this tab.
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    /// Why the settings could not be read or saved, shown in Settings.
    pub settings_error: Option<String>,
}

/// What the network thread for the chosen interface is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkState {
    /// No interface chosen.
    Idle,
    Starting,
    Running {
        controller: EntityId,
    },
    Failed(Failure),
}

#[derive(Debug, Clone)]
pub enum Message {
    InterfacePicked(String),
    ViewPicked(View),
    InspectorToggled,
    VirtualInterfacesToggled,
    SearchChanged(String),
    EntitySelected(EntityId),
    /// A press on a view's empty space: nothing selected.
    SelectionCleared,
    /// Open one of the inspector's tabs.
    InspectorTab(InspectorTab),
    /// A control's slider moved, not yet let go.
    ControlDragged(ControlDrag),
    /// The slider being dragged was let go: its value is sent.
    ControlReleased,
    Rediscover,
    RetryNetwork,
    Copy(String),
    Act(Action),
    /// Several actions, sent in order, such as removing a channel's mapping
    /// before mapping it anew.
    ActInOrder(Vec<Action>),
    DismissNotice,
    MatrixStreams(Streams),
    /// Show another field in the entity list's column at this place, or
    /// remove the column when `None`.
    EntityColumn(usize, Option<EntityField>),
    /// Add a column showing this field at the entity list's right.
    EntityColumnAdded(EntityField),
    /// Move the entity list's column at this place right, or left when
    /// false.
    EntityColumnMoved(usize, bool),
    /// An entity list column dragged to this width.
    EntityColumnResized(Column, f32),
    /// A column's drag ended, so its width is kept.
    EntityColumnResizeEnded,
    /// Fit a column to its text again.
    EntityColumnWidthReset(Column),
    MatrixConnectableOnlyToggled,
    MatrixCollapseToggled(Side, EntityId),
    MatrixHovered(Hover),
    NetworkShows(NetworkShows),
    OverflowToggled,
    NetworkListToggled,
    OverflowClosed,
    NetworkFocused(Option<Focus>),
    /// Start editing a name in the inspector.
    EditName(NameTarget),
    EditChanged(String),
    EditSubmitted,
    EditCancelled,
    /// Open or close a descriptor type in the inspector's tree.
    TreeToggled(EntityId, DescriptorType),
    SettingsOpened,
    SettingsClosed,
    SettingsTab(SettingsTab),
    LanguageSelected(String),
    AppearanceSelected(Appearance),
    SystemAccentToggled(bool),
    /// An accent color picked in Settings, as "#RRGGBB".
    AccentPicked(String),
    AnimationsToggled(bool),
    SystemTheme(iced::theme::Mode),
    SystemAccent(Option<(u8, u8, u8)>),
    AnimationsEnabled(Option<bool>),
    External(External),
    Nothing,
}

impl Triib {
    /// The app with these entities and settings, discovering on `interface`
    /// without a network thread, for drawing it in tests.
    #[cfg(test)]
    pub fn sample(
        settings: Settings,
        interface: Interface,
        entities: BTreeMap<EntityId, DiscoveredEntity>,
        models: BTreeMap<EntityId, EntityModel>,
    ) -> Self {
        Self {
            settings,
            theme: Theme::Dark,
            omarchy_dir: None,
            omarchy: None,
            system_accent: None,
            system_mode: iced::theme::Mode::Dark,
            system_animations: true,
            interfaces: vec![interface],
            search: String::new(),
            network: None,
            generation: 0,
            network_state: NetworkState::Running {
                controller: EntityId(0x9c6b_00ff_fe30_9a2b),
            },
            entities,
            models,
            selected: None,
            pending: Vec::new(),
            notice: None,
            collapsed: BTreeSet::new(),
            hover: Hover::default(),
            neighbor: None,
            network_focus: None,
            overflow_open: false,
            network_list: false,
            editing: None,
            tree_open: BTreeSet::new(),
            control_drag: None,
            inspector_tab: InspectorTab::default(),
            settings_open: false,
            settings_tab: SettingsTab::default(),
            settings_error: None,
        }
    }

    pub fn boot(omarchy_dir: Option<PathBuf>) -> (Self, Task<Message>) {
        let (settings, settings_error) = Settings::load();
        let mut triib = Self {
            settings,
            theme: Theme::Dark,
            omarchy_dir,
            omarchy: None,
            system_accent: None,
            system_mode: iced::theme::Mode::Dark,
            system_animations: true,
            interfaces: avb_net::interfaces(),
            search: String::new(),
            network: None,
            generation: 0,
            network_state: NetworkState::Idle,
            entities: BTreeMap::new(),
            models: BTreeMap::new(),
            selected: None,
            pending: Vec::new(),
            notice: None,
            collapsed: BTreeSet::new(),
            hover: Hover::default(),
            neighbor: None,
            network_focus: None,
            overflow_open: false,
            network_list: false,
            editing: None,
            tree_open: BTreeSet::new(),
            control_drag: None,
            inspector_tab: InspectorTab::default(),
            settings_open: false,
            settings_tab: SettingsTab::default(),
            settings_error,
        };
        triib.reload_omarchy();
        triib.apply_motion();
        triib.start_network();
        let system = iced::system::theme().map(Message::SystemTheme);
        // The portal's own answer, in case it came after iced stopped
        // waiting for it.
        #[cfg(target_os = "linux")]
        let system = Task::batch([
            system,
            Task::perform(desktop::color_scheme(), |mode| match mode {
                Some(mode) => Message::SystemTheme(mode),
                None => Message::Nothing,
            }),
        ]);
        let tasks = Task::batch([
            system,
            Task::perform(desktop::animations_enabled(), Message::AnimationsEnabled),
            Task::perform(desktop::accent_color(), Message::SystemAccent),
        ]);
        (triib, tasks)
    }

    pub fn title(&self) -> String {
        match self.interface() {
            Some(interface) => format!("triib, {}", interface.name),
            None => "triib".to_owned(),
        }
    }

    pub fn theme(&self) -> Theme {
        self.theme.clone()
    }

    /// The chosen interface, when it is present.
    pub fn interface(&self) -> Option<&Interface> {
        let name = self.settings.interface.as_deref()?;
        self.interfaces
            .iter()
            .find(|interface| interface.name == name)
    }

    /// The interfaces the picker offers: physical ones, virtual ones too
    /// unless the filter hides them, and the chosen one always.
    pub fn listed_interfaces(&self) -> impl Iterator<Item = &Interface> {
        let chosen = self.settings.interface.as_deref();
        self.interfaces.iter().filter(move |interface| {
            interface.physical
                || !self.settings.hide_virtual_interfaces
                || chosen == Some(interface.name.as_str())
        })
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InterfacePicked(name) => {
                if self.settings.interface.as_deref() != Some(name.as_str()) {
                    self.settings.interface = Some(name);
                    self.save_settings();
                    self.start_network();
                }
            }
            Message::ViewPicked(view) => {
                self.settings.view = view;
                self.save_settings();
            }
            Message::VirtualInterfacesToggled => {
                self.settings.hide_virtual_interfaces = !self.settings.hide_virtual_interfaces;
                self.save_settings();
            }
            Message::InspectorToggled => {
                self.settings.inspector = !self.settings.inspector;
                self.save_settings();
            }
            Message::SearchChanged(search) => self.search = search,
            Message::EntitySelected(entity_id) => {
                if self
                    .editing
                    .as_ref()
                    .is_some_and(|(target, _)| target.entity != entity_id)
                {
                    self.editing = None;
                }
                self.selected = Some(entity_id);
                if !self.settings.inspector {
                    self.settings.inspector = true;
                    self.save_settings();
                }
            }
            Message::InspectorTab(tab) => self.inspector_tab = tab,
            Message::ControlDragged(drag) => self.control_drag = Some(drag),
            Message::ControlReleased => {
                if let Some(drag) = self.control_drag.take()
                    && let Some(message) = crate::controls_view::released(self, drag)
                {
                    return self.update(message);
                }
            }
            Message::SelectionCleared => {
                self.selected = None;
                self.editing = None;
                self.network_focus = None;
            }
            Message::Rediscover => {
                if let Some(network) = &self.network {
                    network.discover();
                }
            }
            Message::RetryNetwork => self.start_network(),
            Message::Act(action) => {
                if let Some(network) = &self.network {
                    self.notice = None;
                    self.pending.push(action);
                    network.act(action);
                }
            }
            Message::ActInOrder(actions) => {
                if let Some(network) = &self.network {
                    self.notice = None;
                    for action in actions {
                        self.pending.push(action);
                        network.act(action);
                    }
                }
            }
            Message::DismissNotice => self.notice = None,
            Message::EntityColumn(index, field) => {
                let columns = &mut self.settings.entity_columns;
                if index < columns.len() {
                    match field {
                        Some(field) if !columns.contains(&field) => columns[index] = field,
                        Some(_) => {}
                        None => {
                            columns.remove(index);
                        }
                    }
                    self.save_settings();
                }
            }
            Message::EntityColumnMoved(index, right) => {
                let columns = &mut self.settings.entity_columns;
                let other = if right {
                    index.checked_add(1)
                } else {
                    index.checked_sub(1)
                };
                if let Some(other) = other.filter(|&other| other < columns.len())
                    && index < columns.len()
                {
                    columns.swap(index, other);
                    self.save_settings();
                }
            }
            Message::EntityColumnResized(column, width) => match column {
                Column::Name => self.settings.entity_name_width = Some(width),
                Column::Field(field) => {
                    self.settings.entity_column_widths.insert(field, width);
                }
            },
            Message::EntityColumnResizeEnded => self.save_settings(),
            Message::EntityColumnWidthReset(column) => {
                match column {
                    Column::Name => self.settings.entity_name_width = None,
                    Column::Field(field) => {
                        self.settings.entity_column_widths.remove(&field);
                    }
                }
                self.save_settings();
            }
            Message::EntityColumnAdded(field) => {
                if !self.settings.entity_columns.contains(&field) {
                    self.settings.entity_columns.push(field);
                    self.save_settings();
                }
            }
            Message::MatrixStreams(streams) => {
                self.settings.matrix_streams = streams;
                self.save_settings();
            }
            Message::MatrixConnectableOnlyToggled => {
                self.settings.matrix_connectable_only = !self.settings.matrix_connectable_only;
                self.save_settings();
            }
            Message::MatrixCollapseToggled(side, entity_id) => {
                if !self.collapsed.remove(&(side, entity_id)) {
                    self.collapsed.insert((side, entity_id));
                }
            }
            Message::MatrixHovered(hover) => self.hover = hover,
            Message::NetworkShows(shows) => {
                self.settings.network_shows = shows;
                self.save_settings();
                self.network_focus = None;
            }
            Message::NetworkFocused(focus) => {
                // A device brought forward is the one the inspector shows.
                if let Some(Focus::Node(NodeKey::Entity(entity_id))) = focus {
                    self.selected = Some(entity_id);
                }
                self.network_focus = focus;
            }
            Message::EditName(target) => {
                self.editing = Some((target, self.current_name(target)));
                let field = iced::widget::Id::new(crate::view::NAME_FIELD);
                return Task::batch([
                    iced::widget::operation::focus(field.clone()),
                    iced::widget::operation::select_all(field),
                ]);
            }
            Message::EditChanged(text) => {
                // Names longer than an entity holds are not taken.
                if let Some((_, editing)) = &mut self.editing
                    && Name::new(&text).is_some()
                {
                    *editing = text;
                }
            }
            Message::EditSubmitted => {
                if let Some((target, text)) = self.editing.take()
                    && text != self.current_name(target)
                    && let Some(name) = Name::new(&text)
                {
                    return self.update(Message::Act(Action::Rename { target, name }));
                }
            }
            Message::EditCancelled => self.editing = None,
            Message::SettingsOpened => {
                self.settings_open = true;
                self.overflow_open = false;
            }
            Message::SettingsClosed => self.settings_open = false,
            Message::SettingsTab(tab) => self.settings_tab = tab,
            Message::LanguageSelected(tag) => {
                self.settings.language = tag;
                self.save_settings();
            }
            Message::AppearanceSelected(appearance) => {
                self.settings.appearance = appearance;
                self.save_settings();
                self.refresh_theme();
            }
            Message::SystemAccentToggled(enabled) => {
                self.settings.system_accent = enabled;
                self.save_settings();
                self.refresh_theme();
            }
            Message::AccentPicked(hex) => {
                self.settings.accent_color = Some(hex);
                self.save_settings();
                self.refresh_theme();
            }
            Message::AnimationsToggled(enabled) => {
                self.settings.animations = enabled;
                self.save_settings();
                self.apply_motion();
            }
            Message::TreeToggled(entity_id, descriptor_type) => {
                if !self.tree_open.remove(&(entity_id, descriptor_type)) {
                    self.tree_open.insert((entity_id, descriptor_type));
                }
            }
            Message::OverflowToggled => self.overflow_open = !self.overflow_open,
            Message::NetworkListToggled => self.network_list = !self.network_list,
            Message::OverflowClosed => self.overflow_open = false,
            Message::Copy(text) => return iced::clipboard::write(text),
            Message::SystemTheme(mode) => {
                self.system_mode = mode;
                self.refresh_theme();
            }
            Message::SystemAccent(accent) => {
                self.system_accent =
                    accent.map(|(red, green, blue)| Color::from_rgb8(red, green, blue));
                self.refresh_theme();
            }
            Message::AnimationsEnabled(enabled) => {
                if let Some(enabled) = enabled {
                    self.system_animations = enabled;
                    self.apply_motion();
                }
            }
            Message::External(External::OmarchyThemeChanged) => self.reload_omarchy(),
            Message::External(External::Interfaces(interfaces)) => self.interfaces = interfaces,
            Message::External(External::Network(report)) => self.network_report(report),
            Message::Nothing => {}
        }
        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::system::theme_changes().map(Message::SystemTheme),
            Subscription::run(desktop::accent_changes).map(Message::SystemAccent),
            Subscription::run(crate::external_events).map(Message::External),
        ])
    }

    pub fn view(&self) -> Element<'_, Message> {
        crate::view::window(self)
    }

    /// Stops the network thread of the previous interface and starts one
    /// for the chosen interface, forgetting the entities seen before.
    fn start_network(&mut self) {
        self.network = None;
        self.entities.clear();
        self.models.clear();
        self.selected = None;
        self.pending.clear();
        self.hover = Hover::default();
        self.neighbor = None;
        self.network_focus = None;
        let Some(interface) = self.settings.interface.clone() else {
            self.network_state = NetworkState::Idle;
            return;
        };
        self.generation += 1;
        self.network_state = NetworkState::Starting;
        self.network = Some(Network::start(interface, self.generation));
    }

    fn network_report(&mut self, report: Report) {
        if report.generation != self.generation {
            return;
        }
        match report.kind {
            ReportKind::Started { controller } => {
                self.network_state = NetworkState::Running { controller };
            }
            ReportKind::Failed(failure) => {
                self.network = None;
                self.network_state = NetworkState::Failed(failure);
            }
            ReportKind::Online(entity)
            | ReportKind::Changed(entity)
            | ReportKind::Restarted(entity) => {
                self.entities.insert(entity.entity_id(), entity);
            }
            ReportKind::Offline(entity_id, _) => {
                self.entities.remove(&entity_id);
                self.models.remove(&entity_id);
            }
            ReportKind::Model(entity_id, model) => {
                self.models.insert(entity_id, *model);
            }
            ReportKind::Neighbor(neighbor) => self.neighbor = neighbor,
            ReportKind::Finished(action, outcome) => {
                if let Some(position) = self.pending.iter().position(|pending| *pending == action) {
                    self.pending.remove(position);
                }
                self.notice = crate::describe::outcome(self, action, outcome);
            }
        }
    }

    /// The entities matching the search, themselves or by a stream's
    /// name, by name, then entity ID.
    pub fn shown_entities(&self) -> Vec<&DiscoveredEntity> {
        let search = self.search_text();
        let mut shown: Vec<&DiscoveredEntity> = self
            .entities
            .values()
            .filter(|entity| {
                search.is_empty()
                    || self.entity_matches(entity, &search)
                    || self.models.get(&entity.entity_id()).is_some_and(|model| {
                        [true, false].into_iter().any(|input| {
                            model
                                .streams(input)
                                .any(|stream| stream_matches(model, &stream, &search))
                        })
                    })
            })
            .collect();
        shown.sort_by_key(|entity| (self.entity_name(entity).to_lowercase(), entity.entity_id()));
        shown
    }

    /// The search, trimmed and in lower case.
    pub fn search_text(&self) -> String {
        self.search.trim().to_lowercase()
    }

    /// Whether the entity's name, group, IDs or MAC address contain
    /// `search`, in lower case.
    pub fn entity_matches(&self, entity: &DiscoveredEntity, search: &str) -> bool {
        let group = self
            .models
            .get(&entity.entity_id())
            .and_then(EntityModel::entity)
            .map(|descriptor| descriptor.group_name.to_lowercase());
        self.entity_name(entity).to_lowercase().contains(search)
            || group.is_some_and(|group| group.contains(search))
            || entity.entity_id().to_string().contains(search)
            || entity.mac.to_string().contains(search)
            || entity.adp.entity_model_id.to_string().contains(search)
    }

    /// The name to show for an entity by ID.
    pub fn entity_name_of(&self, entity_id: EntityId) -> String {
        match self.entities.get(&entity_id) {
            Some(entity) => self.entity_name(entity),
            None => entity_id.to_string(),
        }
    }

    /// "Entity name: stream name" for a stream of an entity.
    /// A name as the inspector shows it: the one being set when a rename
    /// is on its way, else what the entity holds, falling back for a
    /// descriptor to its localized description.
    pub fn current_name(&self, target: NameTarget) -> String {
        let renaming = self.pending.iter().rev().find_map(|action| match action {
            Action::Rename {
                target: pending,
                name,
            } if *pending == target => Some(name.as_str()),
            _ => None,
        });
        if let Some(name) = renaming {
            return name.to_owned();
        }
        let Some(model) = self.models.get(&target.entity) else {
            return String::new();
        };
        match (target.descriptor_type, target.name_index) {
            (DescriptorType::ENTITY, 0) => model.entity().map(|entity| entity.entity_name),
            (DescriptorType::ENTITY, _) => model.entity().map(|entity| entity.group_name),
            (descriptor_type, _) => model.name_of(descriptor_type, target.index),
        }
        .unwrap_or_default()
        .to_owned()
    }

    /// Whether a change to `entity` is on its way, by what it changes.
    pub fn changing(&self, wanted: impl Fn(&Action) -> bool) -> Option<&Action> {
        self.pending.iter().rev().find(|action| wanted(action))
    }

    pub fn stream_name(
        &self,
        entity_id: EntityId,
        descriptor_type: atdecc::DescriptorType,
        index: u16,
    ) -> String {
        let stream = self
            .models
            .get(&entity_id)
            .and_then(|model| model.name_of(descriptor_type, index))
            .map_or_else(|| format!("stream {index}"), str::to_owned);
        format!("{}: {stream}", self.entity_name_of(entity_id))
    }

    /// The name to show for an entity: the one it was given, else its
    /// entity ID.
    pub fn entity_name(&self, entity: &DiscoveredEntity) -> String {
        self.models
            .get(&entity.entity_id())
            .and_then(EntityModel::entity_name)
            .map_or_else(|| entity.entity_id().to_string(), str::to_owned)
    }

    /// Saves the settings, keeping why when that fails.
    fn save_settings(&mut self) {
        self.settings_error = self.settings.save().err();
    }

    fn reload_omarchy(&mut self) {
        self.omarchy = self.omarchy_dir.as_deref().and_then(omarchy::load);
        self.refresh_theme();
    }

    /// Rebuilds the M3 scheme after a setting, the system mode or the
    /// Omarchy theme changed.
    fn refresh_theme(&mut self) {
        let (seed, dark) = appearance::choose(Inputs {
            appearance: self.settings.appearance,
            system_accent: self.settings.system_accent,
            omarchy: self.omarchy.as_ref(),
            system_accent_color: self.system_accent,
            system_dark: self.system_mode == iced::theme::Mode::Dark,
            chosen_accent: self
                .settings
                .accent_color
                .as_deref()
                .and_then(appearance::hex_to_color)
                .unwrap_or(TRIIB_SEED),
        });
        let name = match (&self.omarchy, self.settings.system_accent) {
            (Some(palette), true) => format!("triib ({})", palette.name),
            _ => "triib".to_owned(),
        };
        self.theme = scheme::theme(name, seed, dark);
    }

    fn apply_motion(&self) {
        motion::set_reduced(!self.settings.animations || !self.system_animations);
    }
}

/// Whether the stream's name contains `search`, in lower case.
pub fn stream_matches(
    model: &EntityModel,
    stream: &atdecc::descriptor::StreamDescriptor<'_>,
    search: &str,
) -> bool {
    model
        .name_of(stream.descriptor_type, stream.index)
        .is_some_and(|name| name.to_lowercase().contains(search))
}
