use crate::action::Action;
use crate::charts::Variant;
use crate::icon;

use hyozu::Data;
use iced::widget::{button, column, container, row, scrollable, text, text_editor};
use iced::{Element, Fill, Font, Length};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

pub struct Detail {
    code: text_editor::Content,
}

#[derive(Debug, Clone)]
pub enum Message {
    Back,
    EditorAction(text_editor::Action),
}

pub enum Instruction {
    Back,
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

impl Detail {
    pub fn new(source: &str) -> Self {
        Self {
            code: text_editor::Content::with_text(source),
        }
    }

    pub fn update(&mut self, message: Message) -> Action<Instruction, Message> {
        match message {
            Message::Back => Action::instruction(Instruction::Back),
            Message::EditorAction(action) => {
                self.code.perform(action);
                Action::none()
            }
        }
    }

    pub fn view<'a>(&'a self, variant: &'a Variant, data: &'a Data, is_dark: bool) -> Element<'a, Message> {
        let back_btn = button(icon::arrow_left().size(16))
            .padding([4, 8])
            .on_press(Message::Back)
            .style(button::text);

        let title = text(variant.name).size(16);
        let top_bar = row![back_btn, title].spacing(8).align_y(iced::Alignment::Center);

        let chart_widget = hyozu::chart(data).height(300).padding(10);

        let highlight_theme = if is_dark {
            iced::highlighter::Theme::Base16Mocha
        } else {
            iced::highlighter::Theme::InspiredGitHub
        };

        let editor = text_editor(&self.code)
            .on_action(Message::EditorAction)
            .highlight("rs", highlight_theme)
            .font(Font::MONOSPACE)
            .size(12);

        let content = column![
            top_bar,
            container(chart_widget).width(Fill),
            scrollable(container(editor).width(Fill).padding([8, 0])).height(Fill),
        ]
        .spacing(8)
        .padding(16)
        .width(Fill)
        .height(Fill);

        container(content).width(Fill).height(Length::Fill).into()
    }
}
