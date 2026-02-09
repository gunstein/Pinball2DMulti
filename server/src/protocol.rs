pub use pinball_shared::protocol::*;

pub fn ball_to_wire(ball: &crate::deep_space::SpaceBall3D) -> BallWire {
    BallWire {
        id: ball.id,
        owner_id: ball.owner_id,
        pos: [round4(ball.pos.x), round4(ball.pos.y), round4(ball.pos.z)],
        axis: [
            round4(ball.axis.x),
            round4(ball.axis.y),
            round4(ball.axis.z),
        ],
        omega: round4(ball.omega),
    }
}

pub fn player_to_wire(player: &crate::player::Player, balls_in_flight: u32) -> PlayerWire {
    PlayerWire {
        id: player.id,
        cell_index: player.cell_index,
        portal_pos: [
            round4(player.portal_pos.x),
            round4(player.portal_pos.y),
            round4(player.portal_pos.z),
        ],
        color: player.color,
        paused: player.paused,
        balls_produced: player.balls_produced,
        balls_in_flight,
    }
}
