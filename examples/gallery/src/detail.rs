use super::*;
use iced::widget::column;

impl App {
    pub(super) fn detail(&self, id: ChartId) -> Element<'_, Message> {
        let v = &self.variants[id.0];
        let back: Element<'_, Message> = control("←  Gallery").style(Ghost).on_press(Message::Back).into();
        let preview = control("Preview").on_press(Message::Source(false));
        let preview: Element<'_, Message> = if self.source {
            preview.style(Ghost).into()
        } else {
            preview.style(Outline).into()
        };
        let code = control("Source").on_press(Message::Source(true));
        let code: Element<'_, Message> = if self.source {
            code.style(Outline).into()
        } else {
            code.style(Ghost).into()
        };
        let mut actions = row![back, spring(), preview, code]
            .spacing(7)
            .align_y(Alignment::Center);
        if matches!(v.build, charts::Builder::Orbital(_)) && !self.source {
            actions = actions
                .push(control("↶").style(Outline).on_press(Message::Orbit(-20.)).into())
                .push(control("↷").style(Outline).on_press(Message::Orbit(20.)).into());
        }
        if self.source {
            actions = actions.push(
                control(if self.copied { "Copied" } else { "Copy code" })
                    .style(Outline)
                    .on_press(Message::Copy)
                    .into(),
            );
        }
        let content: Element<'_, Message> = if self.source {
            {
                let code: Element<'_, Message> = icedcn::widget::code_block(v.source)
                    .language("rs")
                    .copyable(false)
                    .into();
                scrollable(code).height(Fill).boxed()
            }
        } else {
            themer(
                Some(self.theme.iced_theme().clone()),
                hyozu::chart::<Message, iced::Theme>(&self.data[id.0])
                    .on_action(Message::Chart)
                    .style(hyozu::chart::transparent)
                    .padding(18)
                    .height(Fill),
            )
            .boxed()
        };
        let surface: Element<'_, Message> = icedcn::card(content).width(Fill).height(Fill).padding(8).into();
        column![
            actions,
            heading(v.name, 30),
            text(v.category.label()).size(14).color(self.theme.muted.foreground),
            surface
        ]
        .spacing(12)
        .padding(if self.width < 760. { 16 } else { 26 })
        .width(Fill)
        .height(Fill)
        .boxed()
    }
}
