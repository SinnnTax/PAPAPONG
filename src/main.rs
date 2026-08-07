use macroquad::prelude::*;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 600.0;
const PADDLE_W: f32 = 12.0;
const PADDLE_H: f32 = 80.0;
const BALL_SIZE: f32 = 12.0;
const PADDLE_OFFSET: f32 = 20.0;
const PADDLE_SPEED: f32 = 400.0; // pixels per second
const WIN_SCORE: u32 = 5;
const MAX_SPEED_MULTIPLIER: f32 = 3.0;
const PADDLE_SHRINK_AMOUNT: f32 = 12.5;
const PADDLE_MIN_H: f32 = 30.0;
const ABILITY_SIZE: f32 = 24.0;

enum ScoreResult {
    None,
    LeftScored,
    RightScored,
}

#[derive(PartialEq, Clone, Copy)]
enum Side {
    Left,
    Right,
}

#[derive(PartialEq, Clone, Copy)]
enum AbilityType {
    TwicePaddle,
    HalfPaddle,
    SlowBall,
    FastBall,
    FreezeOpponent,
    ReverseControls,
    Shield,
    MultipleBalls,
}

struct Ability {
    rect: Rect,
    kind: AbilityType,
    life_timer: f32,
}

struct ActiveEffect {
    kind: AbilityType,
    target: Side,
    timer: f32,
}

enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    fn config(&self) -> (f32, f32) {
        match self {
            Difficulty::Easy => (200.0, 40.0),
            Difficulty::Medium => (280.0, 20.0),
            Difficulty::Hard => (450.0, 5.0),
        }
    }
}

struct Paddle<'a> {
    rect: Rect,
    texture: &'a Texture2D,
    speed_multiplier: f32,
    speed: f32,
    is_ai: bool,
    deadzone: f32,
    flash_timer: f32,
    original_h: f32,
    is_frozen: bool,
    controls_reversed: bool,
    shield_active: bool,
}

impl<'a> Paddle<'a> {
    fn new(x: f32, texture: &'a Texture2D, speed: f32, is_ai: bool, deadzone: f32) -> Self {
        Self {
            rect: Rect::new(x, WINDOW_H / 2.0 - PADDLE_H / 2.0, PADDLE_W, PADDLE_H),
            texture,
            speed_multiplier: 1.0,
            is_ai,
            deadzone,
            speed,
            flash_timer: 0.0,
            original_h: PADDLE_H,
            is_frozen: false,
            controls_reversed: false,
            shield_active: false,
        }
    }

    fn draw(&self) {
        draw_texture_ex(&self.texture, self.rect.x, self.rect.y, WHITE, DrawTextureParams {
            dest_size: Some(Vec2::new(self.rect.w, self.rect.h)),
            ..Default::default()
        });

        let zone_h = self.rect.h / 3.0;

        draw_rectangle(
            self.rect.x,
            self.rect.y,
            self.rect.w,
            zone_h,
            Color::new(1.0, 0.0, 1.0, 0.2)
        );
        draw_rectangle(
            self.rect.x,
            self.rect.y + zone_h,
            self.rect.w,
            zone_h,
            Color::new(1.0, 1.0, 0.0, 0.2)
        );
        draw_rectangle(
            self.rect.x,
            self.rect.y + 2.0 * zone_h,
            self.rect.w,
            zone_h,
            Color::new(1.0, 0.0, 1.0, 0.2)
        );

        if self.flash_timer > 0.0 {
            draw_rectangle(
                self.rect.x,
                self.rect.y,
                self.rect.w,
                self.rect.h,
                Color::new(1.0, 0.0, 0.0, 0.5)
            );
        }

        if self.shield_active {
            let shield_x = if self.rect.x < WINDOW_W / 2.0 {
                self.rect.x - 5.0
            } else {
                self.rect.x + self.rect.w + 2.0
            };

            draw_rectangle(shield_x, 0.0, 3.0, WINDOW_H, Color::new(0.0, 1.0, 1.0, 0.4));
        }
    }

    fn update(
        &mut self,
        dt: f32,
        going_up_key: KeyCode,
        going_down_key: KeyCode,
        ball_y: Option<f32>
    ) {
        if self.flash_timer > 0.0 {
            self.flash_timer -= dt;
        }

        if self.is_frozen {
            return;
        }

        let (up_key, down_key) = if self.controls_reversed {
            (going_down_key, going_up_key)
        } else {
            (going_up_key, going_down_key)
        };

        if self.is_ai {
            if let Some(target_y) = ball_y {
                let paddle_center = self.rect.y + self.rect.h / 2.0;
                let diff = target_y - paddle_center;

                if diff.abs() > self.deadzone {
                    if diff > 0.0 {
                        self.rect.y += self.speed * dt * self.speed_multiplier;
                    } else {
                        self.rect.y -= self.speed * dt * self.speed_multiplier;
                    }
                }
            }
        } else {
            if is_key_down(down_key) {
                self.rect.y += self.speed * dt * self.speed_multiplier;
            }

            if is_key_down(up_key) {
                self.rect.y -= self.speed * dt * self.speed_multiplier;
            }
        }

        self.rect.y = clamp(self.rect.y, 0.0, WINDOW_H - self.rect.h);
    }

    fn update_speed(&mut self) {
        self.speed_multiplier = (self.speed_multiplier + 0.25).min(MAX_SPEED_MULTIPLIER);
    }

    fn shrink(&mut self) {
        let center_y = self.rect.y + self.rect.h / 2.0;

        self.rect.h = (self.rect.h - PADDLE_SHRINK_AMOUNT).max(PADDLE_MIN_H);

        self.rect.y = center_y - self.rect.h / 2.0;

        self.rect.y = clamp(self.rect.y, 0.0, WINDOW_H - self.rect.h);

        if self.rect.h <= PADDLE_MIN_H {
            self.flash_timer = 3.0;
        }
    }
}

struct Ball {
    rect: Rect,
    vel: Vec2,
    texture: Texture2D,
    speed_multiplier: f32,
    last_hit_by: Option<Side>,
    speed_modifier: f32,
}

impl Ball {
    fn new(texture: Texture2D) -> Self {
        Self {
            rect: Rect::new(
                WINDOW_W / 2.0 - BALL_SIZE / 2.0,
                WINDOW_H / 2.0 - BALL_SIZE / 2.0,
                BALL_SIZE,
                BALL_SIZE
            ),
            vel: Vec2::new(300.0, 220.0),
            texture,
            speed_multiplier: 1.0,
            last_hit_by: None,
            speed_modifier: 1.0,
        }
    }

    fn draw(&self) {
        draw_texture_ex(&self.texture, self.rect.x, self.rect.y, WHITE, DrawTextureParams {
            dest_size: Some(Vec2::new(self.rect.w, self.rect.h)),
            ..Default::default()
        });
    }

    fn update(&mut self, dt: f32) {
        self.rect.x += self.vel.x * dt * self.speed_modifier;
        self.rect.y += self.vel.y * dt * self.speed_modifier;

        // bounce off top wall
        if self.rect.y < 0.0 {
            self.rect.y = 0.0;
            self.vel.y = self.vel.y.abs();
        }
        // bounce off bottom wall
        if self.rect.y + self.rect.h > WINDOW_H {
            self.rect.y = WINDOW_H - self.rect.h;
            self.vel.y = -self.vel.y.abs();
        }
    }

    fn check_paddles(&mut self, left: &Paddle, right: &Paddle) {
        let speed = self.vel.length();

        let max_bounce_angle = (60.0_f32).to_radians();

        if self.rect.overlaps(&left.rect) {
            self.rect.x = left.rect.x + left.rect.w;

            let ball_center_y = self.rect.y + self.rect.h / 2.0;
            let paddle_center_y = left.rect.y + left.rect.h / 2.0;

            let normalized_y = ((ball_center_y - paddle_center_y) / (left.rect.h / 2.0)).clamp(
                -1.0,
                1.0
            );

            let bounce_angle = normalized_y * max_bounce_angle;

            self.vel.x = speed * bounce_angle.cos();
            self.vel.y = speed * bounce_angle.sin();

            self.last_hit_by = Some(Side::Left);
        }

        if self.rect.overlaps(&right.rect) {
            self.rect.x = right.rect.x - self.rect.w;

            let ball_center_y = self.rect.y + self.rect.h / 2.0;
            let paddle_center_y = right.rect.y + right.rect.h / 2.0;

            let normalized_y = ((ball_center_y - paddle_center_y) / (right.rect.h / 2.0)).clamp(
                -1.0,
                1.0
            );
            let bounce_angle = normalized_y * max_bounce_angle;

            self.vel.x = -speed * bounce_angle.cos();
            self.vel.y = speed * bounce_angle.sin();

            self.last_hit_by = Some(Side::Right);
        }
    }

    fn reset(&mut self, direction: Side) {
        self.rect.x = WINDOW_W / 2.0 - BALL_SIZE / 2.0;
        self.rect.y = WINDOW_H / 2.0 - BALL_SIZE / 2.0;

        let dir_x = match direction {
            Side::Right => 1.0,
            Side::Left => -1.0,
        };

        self.vel = Vec2::new(300.0 * dir_x, 220.0) * self.speed_multiplier;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Pong".to_owned(),
        ..Conf::default()
    }
}

fn draw_centre_line() {
    let mut y = 10.0;
    while y < WINDOW_H {
        draw_line(WINDOW_W / 2.0, y, WINDOW_W / 2.0, y + 15.0, 2.0, DARKGRAY);
        y += 25.0;
    }
}

struct Score {
    left: u32,
    right: u32,
}

impl Default for Score {
    fn default() -> Self {
        Self { left: 0, right: 0 }
    }
}

enum GameState {
    Playing,
    GameOver,
    Countdown {
        timer: f32,
    },
    Menu {
        selected: usize,
    },
    Controls {
        in_game: bool,
    },
    DifficultySelect {
        selected: usize,
    },
}

impl Score {
    fn draw(&self) {
        let text = format!("{}   {}", self.left, self.right);
        let dims = measure_text(&text, None, 48, 1.0);
        draw_text(&text, WINDOW_W / 2.0 - dims.width / 2.0, 48.0, 48.0, WHITE);
    }

    fn update(&mut self, ball: &mut Ball, left: &mut Paddle, right: &mut Paddle) -> ScoreResult {
        let left_exit = ball.rect.x + ball.rect.w < 0.0;
        let right_exit = ball.rect.x > WINDOW_W;

        if left_exit {
            if left.shield_active {
                left.shield_active = false;
                ball.rect.x = 0.0;
                ball.vel.x = ball.vel.x.abs();
            } else {
                self.right += 1;
                return ScoreResult::RightScored;
            }
        }

        if right_exit {
            if right.shield_active {
                right.shield_active = false;
                ball.rect.x = WINDOW_W - ball.rect.w;
                ball.vel.x = -ball.vel.x.abs();
            } else {
                self.left += 1;
                return ScoreResult::LeftScored;
            }
        }

        ScoreResult::None
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    /* Run the game loop, stepping the simulation once per frame. */
    let mut score = Score::default();
    let mut game_state = GameState::Menu { selected: 0 };
    let mut winner = "";
    let ball_texture = load_texture("assets/ball.png").await.unwrap();
    let paddle_texture = load_texture("assets/paddle.png").await.unwrap();
    let mut ball = Ball::new(ball_texture);
    let mut left = Paddle::new(PADDLE_OFFSET, &paddle_texture, PADDLE_SPEED, false, 0.0);
    let mut right = Paddle::new(
        WINDOW_W - PADDLE_W - PADDLE_OFFSET,
        &paddle_texture,
        280.0,
        true,
        20.0
    );

    let mut ability_spawn_timer = 8.0;
    let mut floating_ability: Option<Ability> = None;
    let mut active_effect: Option<ActiveEffect> = None;
    let mut extra_balls: Vec<Ball> = Vec::new();

    loop {
        let dt = get_frame_time();

        match game_state {
            GameState::Menu { ref mut selected } => {
                clear_background(BLACK);

                let title = "PONG";
                let title_dims = measure_text(title, None, 80, 1.0);
                draw_text(title, WINDOW_W / 2.0 - title_dims.width / 2.0, 150.0, 80.0, WHITE);

                let options = ["Single Player", "Multi Player", "Controls", "Exit"];

                for (i, option) in options.iter().enumerate() {
                    let color = if i == *selected { YELLOW } else { WHITE };
                    let dims = measure_text(option, None, 40, 1.0);
                    draw_text(
                        option,
                        WINDOW_W / 2.0 - dims.width / 2.0,
                        300.0 + (i as f32) * 50.0,
                        40.0,
                        color
                    );
                }

                if is_key_pressed(KeyCode::Down) {
                    *selected = (*selected + 1).min(options.len() - 1);
                }
                if is_key_pressed(KeyCode::Up) {
                    *selected = if *selected != 0 { *selected - 1 } else { 0 };
                }

                if is_key_pressed(KeyCode::Enter) {
                    match *selected {
                        0 => {
                            game_state = GameState::DifficultySelect { selected: 0 };
                        }
                        1 => {
                            score = Score::default();
                            ball.speed_multiplier = 1.0;
                            ball.reset(Side::Right);
                            left = Paddle::new(
                                PADDLE_OFFSET,
                                &paddle_texture,
                                PADDLE_SPEED,
                                false,
                                0.0
                            );
                            right = Paddle::new(
                                WINDOW_W - PADDLE_W - PADDLE_OFFSET,
                                &paddle_texture,
                                PADDLE_SPEED,
                                false,
                                0.0
                            );

                            floating_ability = None;
                            active_effect = None;
                            ability_spawn_timer = 8.0;
                            extra_balls.clear();

                            game_state = GameState::Countdown { timer: 3.0 };
                        }
                        2 => {
                            game_state = GameState::Controls { in_game: false };
                        }
                        3 => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                }
            }
            GameState::Controls { in_game } => {
                if in_game {
                    draw_centre_line();
                    left.draw();
                    right.draw();
                    ball.draw();
                    for b in extra_balls.iter() {
                        b.draw();
                    }
                    score.draw();

                    draw_rectangle(0.0, 0.0, WINDOW_W, WINDOW_H, Color::new(0.0, 0.0, 0.0, 0.8));
                } else {
                    clear_background(BLACK);
                }

                draw_text("Controls", 100.0, 100.0, 50.0, WHITE);
                draw_text("Left Player:  W (Up), S (Down)", 100.0, 200.0, 30.0, WHITE);
                draw_text("Right Player: Up Arrow, Down Arrow", 100.0, 250.0, 30.0, WHITE);
                draw_text("First to 5 points wins!", 100.0, 300.0, 30.0, GREEN);
                draw_text("Catch abilities for crazy effects!", 100.0, 350.0, 30.0, GOLD);

                let hint = if in_game {
                    "Press ESC to Resume, Press ENTER to Quit to Menu"
                } else {
                    "Press ESC to return to Menu"
                };

                let hdims = measure_text(hint, None, 24, 1.0);
                draw_text(hint, WINDOW_W / 2.0 - hdims.width / 2.0, WINDOW_H - 50.0, 24.0, YELLOW);

                if is_key_pressed(KeyCode::Escape) {
                    if in_game {
                        game_state = GameState::Playing;
                    } else {
                        game_state = GameState::Menu { selected: 2 };
                    }
                }

                if in_game && is_key_pressed(KeyCode::Enter) {
                    game_state = GameState::Menu { selected: 0 };
                }
            }
            GameState::Playing => {
                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::Controls { in_game: true };
                }

                clear_background(BLACK);
                draw_centre_line();

                left.update(dt, KeyCode::W, KeyCode::S, None);
                right.update(dt, KeyCode::Up, KeyCode::Down, Some(ball.rect.y));
                ball.update(dt);
                ball.check_paddles(&left, &right);

                for b in extra_balls.iter_mut() {
                    b.update(dt);
                    b.check_paddles(&left, &right);
                }

                extra_balls = extra_balls
                    .into_iter()
                    .filter(|b| b.rect.x + b.rect.w > 0.0 && b.rect.x < WINDOW_W)
                    .collect();

                for b in extra_balls.iter() {
                    b.draw();
                }

                if floating_ability.is_none() {
                    ability_spawn_timer -= dt;
                    if ability_spawn_timer <= 0.0 {
                        let kind = match rand::gen_range(0, 8) {
                            0 => AbilityType::TwicePaddle,
                            1 => AbilityType::HalfPaddle,
                            2 => AbilityType::SlowBall,
                            3 => AbilityType::FastBall,
                            4 => AbilityType::FreezeOpponent,
                            5 => AbilityType::ReverseControls,
                            6 => AbilityType::Shield,
                            _ => AbilityType::MultipleBalls,
                        };

                        let ability_x = rand::gen_range(WINDOW_W * 0.25, WINDOW_W * 0.75);
                        let ability_y = rand::gen_range(50.0, WINDOW_H - 50.0);
                        floating_ability = Some(Ability {
                            rect: Rect::new(ability_x, ability_y, ABILITY_SIZE, ABILITY_SIZE),
                            kind,
                            life_timer: 8.0,
                        });
                        ability_spawn_timer = 8.0;
                    }
                } else {
                    if let Some(ref mut ability) = floating_ability {
                        ability.life_timer -= dt;
                        if ability.life_timer <= 0.0 {
                            floating_ability = None;
                            ability_spawn_timer = 8.0;
                        }
                    }
                }

                if let Some(ability) = &floating_ability {
                    if ball.rect.overlaps(&ability.rect) {
                        let collector = ball.last_hit_by.unwrap_or(Side::Left);
                        let target = match collector {
                            Side::Left => Side::Right,
                            Side::Right => Side::Left,
                        };

                        match ability.kind {
                            AbilityType::TwicePaddle => {
                                if collector == Side::Left {
                                    left.rect.h = left.original_h * 2.0;
                                } else {
                                    right.rect.h = right.original_h * 2.0;
                                }
                            }
                            AbilityType::HalfPaddle => {
                                if target == Side::Left {
                                    left.rect.h = left.original_h / 2.0;
                                } else {
                                    right.rect.h = right.original_h / 2.0;
                                }
                            }
                            AbilityType::SlowBall => {
                                ball.speed_modifier = 0.5;
                            }
                            AbilityType::FastBall => {
                                ball.speed_modifier = 2.0;
                            }
                            AbilityType::FreezeOpponent => {
                                if target == Side::Left {
                                    left.is_frozen = true;
                                } else {
                                    right.is_frozen = true;
                                }
                            }
                            AbilityType::ReverseControls => {
                                if target == Side::Left {
                                    left.controls_reversed = true;
                                } else {
                                    right.controls_reversed = true;
                                }
                            }
                            AbilityType::Shield => {
                                if collector == Side::Left {
                                    left.shield_active = true;
                                } else {
                                    right.shield_active = true;
                                }
                            }
                            AbilityType::MultipleBalls => {
                                let mut new_ball = Ball::new(ball.texture.clone());
                                new_ball.rect.x = ball.rect.x;
                                new_ball.rect.y = ball.rect.y;
                                new_ball.vel = Vec2::new(ball.vel.x, -ball.vel.y);
                                new_ball.speed_multiplier = ball.speed_multiplier;
                                new_ball.speed_modifier = ball.speed_modifier;
                                new_ball.last_hit_by = ball.last_hit_by;
                                extra_balls.push(new_ball);
                            }
                        }

                        if
                            ability.kind != AbilityType::MultipleBalls &&
                            ability.kind != AbilityType::Shield
                        {
                            let duration = match ability.kind {
                                | AbilityType::TwicePaddle
                                | AbilityType::HalfPaddle
                                | AbilityType::SlowBall => 5.0,
                                AbilityType::FastBall | AbilityType::ReverseControls => 4.0,
                                AbilityType::FreezeOpponent => 3.0,
                                AbilityType::MultipleBalls => 6.0,
                                _ => 5.0,
                            };
                            active_effect = Some(ActiveEffect {
                                kind: ability.kind,
                                target,
                                timer: duration,
                            });
                        }

                        floating_ability = None;
                    }
                }

                if let Some(ref mut effect) = active_effect {
                    effect.timer -= dt;
                    if effect.timer <= 0.0 {
                        match effect.kind {
                            AbilityType::TwicePaddle => {
                                if effect.target == Side::Right {
                                    left.rect.h = left.original_h;
                                } else {
                                    right.rect.h = right.original_h;
                                }
                            }
                            AbilityType::HalfPaddle => {
                                if effect.target == Side::Left {
                                    left.rect.h = left.original_h;
                                } else {
                                    right.rect.h = right.original_h;
                                }
                            }
                            AbilityType::SlowBall | AbilityType::FastBall => {
                                ball.speed_modifier = 1.0;
                            }
                            AbilityType::FreezeOpponent => {
                                left.is_frozen = false;
                                right.is_frozen = false;
                            }
                            AbilityType::ReverseControls => {
                                left.controls_reversed = false;

                                right.controls_reversed = false;
                            }
                            AbilityType::MultipleBalls => {
                                extra_balls.clear();
                            }
                            _ => {}
                        }
                        active_effect = None;
                    }
                }

                if let Some(ability) = &floating_ability {
                    let color = match ability.kind {
                        AbilityType::TwicePaddle => GREEN,
                        AbilityType::HalfPaddle => YELLOW,
                        AbilityType::SlowBall => BLUE,
                        AbilityType::FastBall => RED,
                        AbilityType::FreezeOpponent => SKYBLUE,
                        AbilityType::ReverseControls => PURPLE,
                        AbilityType::Shield => WHITE,
                        AbilityType::MultipleBalls => ORANGE,
                    };
                    draw_rectangle(
                        ability.rect.x,
                        ability.rect.y,
                        ability.rect.w,
                        ability.rect.h,
                        color
                    );
                }

                match score.update(&mut ball, &mut left, &mut right) {
                    ScoreResult::LeftScored => {
                        right.shrink();
                        ball.reset(Side::Left);

                        ball.speed_multiplier = (ball.speed_multiplier + 0.4).min(
                            MAX_SPEED_MULTIPLIER
                        );

                        left.update_speed();
                        right.update_speed();

                        if score.left >= WIN_SCORE {
                            winner = "Left player wins!";
                            game_state = GameState::GameOver;
                        } else if score.right >= WIN_SCORE {
                            winner = "Right player wins!";
                            game_state = GameState::GameOver;
                        } else {
                            game_state = GameState::Countdown { timer: 3.0 };
                        }
                    }
                    ScoreResult::RightScored => {
                        left.shrink();
                        ball.reset(Side::Right);

                        ball.speed_multiplier = (ball.speed_multiplier + 0.4).min(
                            MAX_SPEED_MULTIPLIER
                        );

                        left.update_speed();
                        right.update_speed();

                        if score.right >= WIN_SCORE {
                            winner = "Right player wins!";
                            game_state = GameState::GameOver;
                        } else {
                            game_state = GameState::Countdown { timer: 3.0 };
                        }
                    }
                    ScoreResult::None => {}
                }

                left.draw();
                right.draw();
                ball.draw();
                score.draw();
            }
            GameState::GameOver => {
                let dims = measure_text(winner, None, 48, 1.0);
                draw_text(winner, WINDOW_W / 2.0 - dims.width / 2.0, WINDOW_H / 2.0, 48.0, WHITE);

                let hint = "Press Enter to return to Menu";
                let hdims = measure_text(hint, None, 24, 1.0);
                draw_text(
                    hint,
                    WINDOW_W / 2.0 - hdims.width / 2.0,
                    WINDOW_H / 2.0 + 40.0,
                    24.0,
                    GRAY
                );

                if is_key_pressed(KeyCode::Enter) {
                    game_state = GameState::Menu { selected: 0 };
                }
            }
            GameState::Countdown { ref mut timer } => {
                clear_background(BLACK);
                draw_centre_line();

                left.update(dt, KeyCode::W, KeyCode::S, None);
                right.update(dt, KeyCode::Up, KeyCode::Down, Some(ball.rect.y));

                ball.draw();

                *timer -= dt;

                if *timer <= 0.0 {
                    game_state = GameState::Playing;
                } else {
                    let seconds_left = (*timer).ceil();
                    let text = format!("{}", seconds_left);

                    let progress = 1.0 - (*timer % 1.0);
                    let font_size = 150.0 - progress * 75.0;

                    let dims = measure_text(&text, None, font_size as u16, 1.0);
                    draw_text(
                        &text,
                        WINDOW_W / 2.0 - dims.width / 2.0,
                        WINDOW_H / 2.0 + font_size / 2.0 - 25.0,
                        font_size,
                        WHITE
                    );
                }

                left.draw();
                right.draw();

                score.draw();
            }
            GameState::DifficultySelect { ref mut selected } => {
                clear_background(BLACK);

                let title = "SELECT DIFFICULTY";
                let title_dims = measure_text(title, None, 60, 1.0);
                draw_text(title, WINDOW_W / 2.0 - title_dims.width / 2.0, 150.0, 60.0, WHITE);

                let options = ["Easy", "Medium", "Hard", "Back"];

                for (i, option) in options.iter().enumerate() {
                    let color = if i == *selected { YELLOW } else { WHITE };
                    let dims = measure_text(option, None, 40, 1.0);
                    draw_text(
                        option,
                        WINDOW_W / 2.0 - dims.width / 2.0,
                        300.0 + (i as f32) * 50.0,
                        40.0,
                        color
                    );
                }

                if is_key_pressed(KeyCode::Down) {
                    *selected = (*selected + 1).min(options.len() - 1);
                }
                if is_key_pressed(KeyCode::Up) {
                    *selected = if *selected != 0 { *selected - 1 } else { 0 };
                }

                if is_key_pressed(KeyCode::Enter) {
                    if *selected == 3 {
                        game_state = GameState::Menu { selected: 0 };
                    } else {
                        let difficulty = match *selected {
                            0 => Difficulty::Easy,
                            1 => Difficulty::Medium,
                            _ => Difficulty::Hard,
                        };
                        let (ai_speed, ai_deadzone) = difficulty.config();

                        score = Score::default();
                        ball.speed_multiplier = 1.0;
                        ball.reset(Side::Right);
                        left = Paddle::new(
                            PADDLE_OFFSET,
                            &paddle_texture,
                            PADDLE_SPEED,
                            false,
                            0.0
                        );
                        right = Paddle::new(
                            WINDOW_W - PADDLE_W - PADDLE_OFFSET,
                            &paddle_texture,
                            ai_speed,
                            true,
                            ai_deadzone
                        );

                        floating_ability = None;
                        active_effect = None;
                        ability_spawn_timer = 8.0;
                        extra_balls.clear();

                        game_state = GameState::Countdown { timer: 3.0 };
                    }
                }

                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::Menu { selected: 0 };
                }
            }
        }
        next_frame().await;
    }
}
