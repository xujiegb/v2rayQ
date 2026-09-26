use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Cell, Row, Table},
};

use crate::model::{Node, Protocol};

#[derive(Debug, Default)]
pub struct NodesPage {
    nodes: Vec<Node>,
}

impl NodesPage {
    pub fn set_nodes(&mut self, nodes: Vec<Node>) {
        self.nodes = nodes;
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let header = Row::new([
            Cell::from("Name"),
            Cell::from("Protocol"),
            Cell::from("Server"),
            Cell::from("Port"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));

        let rows = self.nodes.iter().map(|node| {
            Row::new([
                Cell::from(node.name.clone()),
                Cell::from(protocol_name(&node.protocol)),
                Cell::from(node.endpoint.host.clone()),
                Cell::from(node.endpoint.port.to_string()),
            ])
        });

        let table = Table::new(
            rows,
            [
                Constraint::Percentage(35),
                Constraint::Length(14),
                Constraint::Percentage(35),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .block(Block::bordered().title("Nodes"))
        .column_spacing(1);

        frame.render_widget(table, area);
    }
}

fn protocol_name(protocol: &Protocol) -> &'static str {
    match protocol {
        Protocol::Vless(_) => "VLESS",
        Protocol::Vmess(_) => "VMess",
        Protocol::Trojan(_) => "Trojan",
        Protocol::Shadowsocks(_) => "Shadowsocks",
        Protocol::AnyTls(_) => "AnyTLS",
        Protocol::Hysteria2(_) => "Hysteria2",
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::model::{Endpoint, Protocol, Vless};

    #[test]
    fn stores_nodes() {
        let node = Node {
            id: Uuid::new_v4(),
            name: "Example".to_owned(),
            endpoint: Endpoint::new("example.com", 443),
            protocol: Protocol::Vless(Vless {
                uuid: Uuid::new_v4(),
                flow: None,
                transport: None,
                tls: None,
            }),
        };

        let mut page = NodesPage::default();
        page.set_nodes(vec![node]);

        assert_eq!(page.nodes().len(), 1);
        assert_eq!(page.nodes()[0].name, "Example");
    }
}
