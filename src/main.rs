#![windows_subsystem = "windows"]

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use home::home_dir;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::container::Appearance;
use iced::widget::text::Shaping;
use iced::widget::{Button, Column, Container, Row, Scrollable, Space, Text, TextInput};
use iced::{executor, theme, Alignment, Application, Background, Color, Command, Element, Length, Settings, Theme, window};
use iced_aw::floating_element::{Anchor, Offset};
use iced_aw::{FloatingElement, TabBar, TabLabel};
use rust_fuzzy_search::fuzzy_compare;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use iced::window::Position;

pub fn main() -> iced::Result {
    //window::icon::from_rgba()
    Macros::run(Settings {
        id: None,
        window: window::Settings {
            size: (1000, 600),
            position: Position::Centered,
            min_size: None,
            max_size: None,
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            level: Default::default(),
            icon: None,
            platform_specific: Default::default(),
        },
        flags: (),
        default_font: Default::default(),
        default_text_size: 16.0,
        antialiasing: true,
        exit_on_close_request: true,
    })
}

#[derive(Debug, Clone)]
pub enum Message {
    ChangeTab(usize),
    ChangeSearchText(String),
    AddFood(AddFood),
    AddFoodNameChanged(String),
    AddFoodServingSizeChanged(String),
    AddFoodCarbsChanged(String),
    AddFoodFatsChanged(String),
    AddFoodProteinsChanged(String),
    FoodServingCurrentServingSizeChanged(u32, String),
    AddFeedEntry(Food),
    ModifyFood(Food),
    DeleteFood(Food),
    DeleteFeedEntry(u32),
}

#[derive(Debug, Copy, Clone)]
pub enum AddFood {
    Cancel,
    BeginAdd,
    FinishAdd,
}

pub struct Tab {
    title: String,
    tab_type: TabType,
}

pub enum TabType {
    Feed,
    Food,
}

struct Macros {
    tabs: Vec<Tab>,
    current_tab: usize,
    search_text: String,
    adding_food: bool,
    add_food_name: String,
    add_food_carbs: String,
    add_food_serving_size: String,
    add_food_fats: String,
    add_food_proteins: String,
    foods: Vec<Food>,
    feed: Vec<FeedEntry>,
    config_path: PathBuf,
    next_food_id: u32,
    next_feed_id: u32,
}

impl Application for Macros {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let config_path = home_dir().unwrap_or(PathBuf::from("")).join(".macros");
        fs::create_dir_all(&config_path).expect("Failed to create config directory");
        let foods =
            fs::read_to_string(config_path.join("foods.json")).unwrap_or(String::from("[]"));
        let mut foods =
            serde_json::from_str::<Vec<Food>>(&foods).expect("Failed to parse foods.json");
        let next_food_id = foods.iter().fold(0, |acc, food| max(acc, food.id + 1));
        foods
            .iter_mut()
            .for_each(|food| food.current_serving_size = String::from("1.0"));

        let feed = fs::read_to_string(config_path.join("feed.json")).unwrap_or(String::from("[]"));
        let mut feed =
            serde_json::from_str::<Vec<FeedEntry>>(&feed).expect("Failed to parse feed.json");
        let next_feed_id = feed.iter().fold(0, |acc, food| max(acc, food.id + 1));
        feed.sort_by_key(|entry| entry.date);
        (
            Macros {
                tabs: vec![
                    Tab {
                        title: String::from("Feed"),
                        tab_type: TabType::Feed,
                    },
                    Tab {
                        title: String::from("Food"),
                        tab_type: TabType::Food,
                    },
                ],
                current_tab: 0,
                search_text: "".to_string(),
                adding_food: false,
                add_food_name: "".to_string(),
                add_food_carbs: "".to_string(),
                add_food_serving_size: "".to_string(),
                add_food_fats: "".to_string(),
                add_food_proteins: "".to_string(),
                foods,
                feed,
                config_path,
                next_food_id,
                next_feed_id,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Macros")
    }

    fn update(&mut self, message: Message) -> Command<Self::Message> {
        match message {
            Message::ChangeTab(index) => {
                if index < self.tabs.len() {
                    self.current_tab = index;
                } else {
                    panic!("Invalid tab index")
                }
            }
            Message::ChangeSearchText(new_search_text) => {
                self.search_text = new_search_text;
                self.foods.iter_mut().for_each(|food| {
                    food.relevance =
                        fuzzy_compare(&food.name.to_lowercase(), &self.search_text.to_lowercase())
                            + if food
                                .name
                                .to_lowercase()
                                .contains(&self.search_text.to_lowercase())
                            {
                                1.0
                            } else {
                                0.0
                            };
                });
                self.foods.sort_by(|a, b| {
                    b.relevance
                        .partial_cmp(&a.relevance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            Message::AddFood(adding_food) => {
                match adding_food {
                    AddFood::Cancel => {
                        self.adding_food = false;
                    }
                    AddFood::BeginAdd => {
                        self.next_food_id =
                            self.foods.iter().fold(0, |acc, food| max(acc, food.id + 1));
                        self.adding_food = true;
                    }
                    AddFood::FinishAdd => {
                        if self.add_food_name.trim().is_empty()
                            || self.add_food_serving_size.trim().is_empty()
                            || self.add_food_carbs.parse::<f32>().is_err()
                            || self.add_food_fats.parse::<f32>().is_err()
                            || self.add_food_proteins.parse::<f32>().is_err()
                        {
                            return Command::none();
                        }

                        self.adding_food = false;

                        let new_food = Food {
                            id: self.next_food_id,
                            name: self.add_food_name.clone(),
                            brand: "".to_string(),
                            carbs: self.add_food_carbs.clone().parse().unwrap(),
                            fats: self.add_food_fats.clone().parse().unwrap(),
                            protein: self.add_food_proteins.clone().parse().unwrap(),
                            serving_size: self.add_food_serving_size.to_string(),
                            current_serving_size: String::from("1.0"),
                            relevance: fuzzy_compare(
                                &self.add_food_name.to_lowercase(),
                                &self.search_text.to_lowercase(),
                            ) + if self
                                .add_food_name
                                .to_lowercase()
                                .contains(&self.search_text.to_lowercase())
                            {
                                1.0
                            } else {
                                0.0
                            },
                        };

                        if new_food.carbs < 0.0
                            || new_food.carbs > 1000.0
                            || new_food.fats < 0.0
                            || new_food.fats > 1000.0
                            || new_food.protein < 0.0
                            || new_food.protein > 1000.0
                        {
                            return Command::none();
                        }
                        self.foods = self
                            .foods
                            .iter()
                            .filter(|food| food.id != new_food.id)
                            .cloned()
                            .collect();
                        self.foods.push(new_food);
                        let foods_str = serde_json::to_string_pretty(&self.foods)
                            .expect("Failed to serialize foods");
                        fs::write(self.config_path.join("foods.json"), foods_str)
                            .expect("Failed to write foods.json");

                        self.foods.iter_mut().for_each(|food| {
                            food.relevance = fuzzy_compare(
                                &food.name.to_lowercase(),
                                &self.search_text.to_lowercase(),
                            ) + if food
                                .name
                                .to_lowercase()
                                .contains(&self.search_text.to_lowercase())
                            {
                                1.0
                            } else {
                                0.0
                            };
                        });
                        self.foods.sort_by(|a, b| {
                            b.relevance
                                .partial_cmp(&a.relevance)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                }
                self.add_food_name = "".to_string();
                self.add_food_serving_size = "".to_string();
                self.add_food_carbs = "".to_string();
                self.add_food_fats = "".to_string();
                self.add_food_proteins = "".to_string();
            }
            Message::AddFoodNameChanged(new_add_food_name) => {
                self.add_food_name = new_add_food_name;
            }
            Message::AddFoodCarbsChanged(new_add_food_carbs) => {
                self.add_food_carbs = new_add_food_carbs;
            }
            Message::AddFoodFatsChanged(new_add_food_fats) => {
                self.add_food_fats = new_add_food_fats;
            }
            Message::AddFoodProteinsChanged(new_add_food_proteins) => {
                self.add_food_proteins = new_add_food_proteins
            }
            Message::AddFoodServingSizeChanged(new_add_food_serving_size) => {
                self.add_food_serving_size = new_add_food_serving_size;
            }
            Message::FoodServingCurrentServingSizeChanged(food_id, new_current_serving_size) => {
                if let Some(food) = self.foods.iter_mut().find(|food| food.id == food_id) {
                    food.current_serving_size = new_current_serving_size;
                }
            }
            Message::AddFeedEntry(food) => {
                let amount = food.current_serving_size.parse::<f32>();

                if let Ok(amount) = amount {
                    if amount < 0.0 || amount > 1000.0 {
                        return Command::none();
                    }

                    self.feed.push(FeedEntry {
                        id: self.next_feed_id,
                        food_id: food.id,
                        amount,
                        date: Utc::now(),
                        is_daily_total: false,
                        carbs: 0.0,
                        fats: 0.0,
                        protein: 0.0,
                    });
                    self.next_feed_id += 1;

                    self.feed.sort_by_key(|entry| entry.date);

                    let feed_str =
                        serde_json::to_string_pretty(&self.feed).expect("Failed to serialize feed");
                    fs::write(self.config_path.join("feed.json"), feed_str)
                        .expect("Failed to write feed.json");
                }
            }
            Message::ModifyFood(food) => {
                self.next_food_id = food.id;
                self.add_food_name = food.name.clone();
                self.add_food_carbs = food.carbs.to_string();
                self.add_food_fats = food.fats.to_string();
                self.add_food_proteins = food.protein.to_string();
                self.add_food_serving_size = food.serving_size.clone();
                self.adding_food = true;
            }
            Message::DeleteFood(food) => {
                self.foods = self
                    .foods
                    .iter()
                    .filter(|f| f.id != food.id)
                    .cloned()
                    .collect();
                self.feed = self
                    .feed
                    .iter()
                    .filter(|f| f.food_id != food.id)
                    .cloned()
                    .collect();

                let foods_str =
                    serde_json::to_string_pretty(&self.foods).expect("Failed to serialize foods");
                fs::write(self.config_path.join("foods.json"), foods_str)
                    .expect("Failed to write foods.json");

                let feed_str =
                    serde_json::to_string_pretty(&self.feed).expect("Failed to serialize feed");
                fs::write(self.config_path.join("feed.json"), feed_str)
                    .expect("Failed to write feed.json");
            }
            Message::DeleteFeedEntry(feed_entry_id) => {
                self.feed = self
                    .feed
                    .iter()
                    .filter(|f| f.id != feed_entry_id)
                    .cloned()
                    .collect();

                let feed_str =
                    serde_json::to_string_pretty(&self.feed).expect("Failed to serialize feed");
                fs::write(self.config_path.join("feed.json"), feed_str)
                    .expect("Failed to write feed.json");
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        Column::with_children(vec![
            TabBar::with_tab_labels(
                self.tabs
                    .iter()
                    .enumerate()
                    .map(|(index, tab)| (index, TabLabel::Text(tab.title.clone())))
                    .collect(),
                Message::ChangeTab,
            )
            .set_active_tab(&self.current_tab)
            .into(),
            self.main_content(),
        ])
        .into()
    }

    fn theme(&self) -> Theme {
        Theme::default()
    }
}

impl Macros {
    fn main_content(&self) -> Element<Message> {
        match self.tabs[self.current_tab].tab_type {
            TabType::Feed => self.feed(),
            TabType::Food => {
                if self.adding_food {
                    self.add_food() //.explain(Color::new(1.0, 0.0, 0.0, 1.0))
                } else {
                    self.food_tab()
                }
            }
        }
    }

    fn feed(&self) -> Element<Message> {
        let mut feed = self.feed.clone();
        let mut macro_map: HashMap<String, (f32, f32, f32)> = HashMap::new();
        for entry in self.feed.iter() {
            let food = self
                .foods
                .iter()
                .find(|food| food.id == entry.food_id)
                .unwrap();

            let day = DateTime::<Local>::from(entry.date)
                .format("%Y-%m-%d")
                .to_string();
            let def: (f32, f32, f32) = (0.0, 0.0, 0.0);
            let daily_total = macro_map.get(&day).unwrap_or(&def);
            macro_map.insert(
                day,
                (
                    daily_total.0 + food.carbs * entry.amount,
                    daily_total.1 + food.fats * entry.amount,
                    daily_total.2 + food.protein * entry.amount,
                ),
            );
        }

        for (key, value) in macro_map {
            let from: NaiveDateTime =
                NaiveDateTime::parse_from_str(&format!("{} 23:59:59", key), "%Y-%m-%d %H:%M:%S")
                    .unwrap();
            let date_time = Local.from_local_datetime(&from).unwrap();
            feed.push(FeedEntry {
                id: 0,
                food_id: 0,
                amount: 0.0,
                date: DateTime::<Utc>::from(date_time),
                // %Y-%m-%d %H:%M:%S
                is_daily_total: true,
                carbs: value.0,
                fats: value.1,
                protein: value.2,
            });
        }

        feed.sort_by_key(|entry| entry.date);

        Scrollable::new(Column::with_children(
            feed.iter()
                .rev()
                .enumerate()
                .map(|(index, feed_entry)| {
                    Container::new(if feed_entry.is_daily_total {
                        Row::with_children(vec![
                            Text::new(format!(
                                "{}",
                                DateTime::<Local>::from(feed_entry.date).format("%Y-%m-%d")
                            ))
                            .width(300)
                            .size(20)
                            .into(),
                            Space::new(20, 10).into(),
                            Text::new("-").size(20).into(),
                            Space::new(20, 10).into(),
                            Text::new(format!(
                                "Carbs: {:.1} Fats: {:.1} Proteins: {:.1} Calories: {:.1}",
                                feed_entry.carbs,
                                feed_entry.fats,
                                feed_entry.protein,
                                feed_entry.carbs * 4.0
                                    + feed_entry.fats * 9.0
                                    + feed_entry.protein * 4.0
                            ))
                            .size(20)
                            .into(),
                        ])
                    } else {
                        let food = self
                            .foods
                            .iter()
                            .find(|food| food.id == feed_entry.food_id)
                            .unwrap();

                        Row::with_children(vec![
                            Row::with_children(vec![
                                Text::new(format!(
                                "{}",
                                DateTime::<Local>::from(feed_entry.date).format("%H:%M ")
                            )).width(50).into(),
                                Text::new(&food.name).width(250).into(),
                                Space::new(20, 10).into(),
                                Text::new("-").into(),
                                Space::new(20, 10).into(),
                                Text::new(format!(
                                    "Servings: {} Carbs: {:.1} Fats: {:.1} Proteins: {:.1} Calories: {:.1}",
                                    feed_entry.amount,
                                    food.carbs * feed_entry.amount,
                                    food.fats * feed_entry.amount,
                                    food.protein * feed_entry.amount,
                                    food.calories() * feed_entry.amount
                                ))
                                .into(),
                            ])
                            .width(Length::FillPortion(95))
                            .into(),
                            Button::new(Text::new("🗑").shaping(Shaping::Advanced))
                                .width(Length::Shrink)
                                .on_press(Message::DeleteFeedEntry(feed_entry.id))
                                .style(theme::Button::Destructive)
                                .into(),
                        ])
                    })
                    .width(Length::Fill)
                    .padding(10)
                    .style(move |_theme: &Theme| {
                        if index % 2 == 0 {
                            Appearance {
                                text_color: None,
                                background: Some(Background::Color(Color::new(0.9, 0.9, 0.9, 1.0))),
                                border_radius: Default::default(),
                                border_width: 0.0,
                                border_color: Default::default(),
                            }
                        } else {
                            Appearance {
                                text_color: None,
                                background: Some(Background::Color(Color::new(
                                    0.95, 0.95, 0.95, 1.0,
                                ))),
                                border_radius: Default::default(),
                                border_width: 0.0,
                                border_color: Default::default(),
                            }
                        }
                    })
                    .into()
                })
                .collect(),
        ))
        .into()
    }

    fn food_tab(&self) -> Element<Message> {
        Column::with_children(vec![
            TextInput::new("Find Food...", &self.search_text)
                .on_input(Message::ChangeSearchText)
                .into(),
            FloatingElement::new(
                Scrollable::new(Column::with_children(
                    self.foods
                        .iter()
                        .enumerate()
                        .map(|(index, food)| {
                            Container::new(
                                Row::with_children(vec![
                                    Row::with_children(vec![
                                        Text::new(&food.name).width(300).into(),
                                        Space::new(20, 10).into(),
                                        Text::new("-").into(),
                                        Space::new(20, 10).into(),
                                        Text::new(format!(
                                            "Serving Size: {} Carbs: {} Fats: {:.1} Proteins: {:.1} Calories: {:.1}",
                                            food.serving_size,
                                            food.carbs,
                                            food.fats,
                                            food.protein,
                                            food.calories()
                                        ))
                                        .into(),
                                    ])
                                    .width(Length::FillPortion(5))
                                    .align_items(Alignment::Center)
                                    .into(),
                                    Row::with_children(vec![
                                        TextInput::new("Serving Size", &food.current_serving_size)
                                            .on_input(|new_str| {
                                                Message::FoodServingCurrentServingSizeChanged(
                                                    food.id, new_str,
                                                )
                                            })
                                            .into(),
                                        Button::new(Text::new("＋").shaping(Shaping::Advanced))
                                            .on_press(Message::AddFeedEntry(food.clone()))
                                            .style(theme::Button::Positive)
                                            .into(),
                                        Button::new(Text::new("⚙").shaping(Shaping::Advanced))
                                            .on_press(Message::ModifyFood(food.clone()))
                                            .into(),
                                        Button::new(Text::new("🗑").shaping(Shaping::Advanced))
                                            .on_press(Message::DeleteFood(food.clone()))
                                            .style(theme::Button::Destructive)
                                            .into(),
                                    ])
                                    .spacing(2.0)
                                    .width(Length::FillPortion(1))
                                    .into(),
                                ])
                                .align_items(Alignment::Center),
                            )
                            .width(Length::Fill)
                            .padding(10)
                            .style(move |_theme: &Theme| {
                                if index % 2 == 0 {
                                    Appearance {
                                        text_color: None,
                                        background: Some(Background::Color(Color::new(
                                            0.9, 0.9, 0.9, 1.0,
                                        ))),
                                        border_radius: Default::default(),
                                        border_width: 0.0,
                                        border_color: Default::default(),
                                    }
                                } else {
                                    Appearance {
                                        text_color: None,
                                        background: Some(Background::Color(Color::new(
                                            0.95, 0.95, 0.95, 1.0,
                                        ))),
                                        border_radius: Default::default(),
                                        border_width: 0.0,
                                        border_color: Default::default(),
                                    }
                                }
                            })
                            .into()
                        })
                        .collect(),
                ))
                .width(Length::Fill)
                .height(Length::Fill),
                Button::new(Text::new("Create Food").shaping(Shaping::Advanced)).on_press(Message::AddFood(AddFood::BeginAdd)),
            )
            .anchor(Anchor::SouthEast)
            .offset(Offset::from(20.0))
            .into(),
        ])
        .into()
    }

    fn add_food(&self) -> Element<Message> {
        const LABEL_WIDTH: u16 = 100;
        Container::new(
            Column::with_children(vec![
                Row::with_children(vec![
                    Text::new("Name:").width(LABEL_WIDTH).into(),
                    TextInput::new("Enter Name", &self.add_food_name)
                        .on_input(Message::AddFoodNameChanged)
                        .into(),
                ])
                .align_items(Alignment::Center)
                .spacing(15)
                .width(300)
                .into(),
                Row::with_children(vec![
                    Text::new("Serving Size:").width(LABEL_WIDTH).into(),
                    TextInput::new("Enter Serving Size", &self.add_food_serving_size)
                        .on_input(Message::AddFoodServingSizeChanged)
                        .into(),
                ])
                .align_items(Alignment::End)
                .spacing(15)
                .width(300)
                .into(),
                Row::with_children(vec![
                    Text::new("Carbs:").width(LABEL_WIDTH).into(),
                    TextInput::new("Enter Carbs", &self.add_food_carbs)
                        .on_input(Message::AddFoodCarbsChanged)
                        .into(),
                ])
                .align_items(Alignment::End)
                .spacing(15)
                .width(300)
                .into(),
                Row::with_children(vec![
                    Text::new("Fats:").width(LABEL_WIDTH).into(),
                    TextInput::new("Enter Fats", &self.add_food_fats)
                        .on_input(Message::AddFoodFatsChanged)
                        .into(),
                ])
                .align_items(Alignment::End)
                .spacing(15)
                .width(300)
                .into(),
                Row::with_children(vec![
                    Text::new("Proteins:").width(LABEL_WIDTH).into(),
                    TextInput::new("Enter Proteins", &self.add_food_proteins)
                        .on_input(Message::AddFoodProteinsChanged)
                        .into(),
                ])
                .align_items(Alignment::End)
                .spacing(15)
                .width(300)
                .into(),
                Column::with_children(vec![Row::with_children(vec![
                    Button::new("Cancel")
                        .on_press(Message::AddFood(AddFood::Cancel))
                        .style(theme::Button::Destructive)
                        .into(),
                    Button::new("Add")
                        .on_press(Message::AddFood(AddFood::FinishAdd))
                        .style(theme::Button::Positive)
                        .into(),
                ])
                .align_items(Alignment::Center)
                .spacing(30)
                .into()])
                .width(300)
                .align_items(Alignment::Center)
                .into(),
            ])
            .spacing(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .padding(20)
        .into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Food {
    id: u32,
    name: String,
    brand: String,
    carbs: f32,
    fats: f32,
    protein: f32,
    serving_size: String,

    #[serde(skip_serializing, skip_deserializing)]
    current_serving_size: String,
    #[serde(skip_serializing, skip_deserializing)]
    relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeedEntry {
    id: u32,
    food_id: u32,
    amount: f32,
    date: DateTime<Utc>,

    #[serde(skip_serializing, skip_deserializing)]
    is_daily_total: bool,
    #[serde(skip_serializing, skip_deserializing)]
    carbs: f32,
    #[serde(skip_serializing, skip_deserializing)]
    fats: f32,
    #[serde(skip_serializing, skip_deserializing)]
    protein: f32,
}

impl Food {
    fn calories(&self) -> f32 {
        self.carbs * 4.0 + self.fats * 9.0 + self.protein * 4.0
    }
}
