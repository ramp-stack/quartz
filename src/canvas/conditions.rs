use super::core::Canvas;
use crate::value::{Value, Expr, resolve_expr, compare_operands};
use crate::expr::parse_condition;
use crate::types::Condition;

impl Canvas {
    pub(crate) fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::KeyHeld(k)    =>  self.input.held_keys.contains(k),
            Condition::KeyNotHeld(k) => !self.input.held_keys.contains(k),
            Condition::Collision(t) => {
                self.store.get_indices(t).iter().any(|&i| {
                    (0..self.store.objects.len()).any(|j| {
                        if i == j { return false; }
                        match (self.store.objects.get(i), self.store.objects.get(j)) {
                            (Some(a), Some(b)) => Self::check_collision(a, b),
                            _ => false,
                        }
                    })
                })
            }
            Condition::NoCollision(t) => !self.evaluate_condition(&Condition::Collision(t.clone())),
            Condition::And(c1, c2) => self.evaluate_condition(c1) && self.evaluate_condition(c2),
            Condition::Or(c1, c2)  => self.evaluate_condition(c1) || self.evaluate_condition(c2),
            Condition::Not(c)      => !self.evaluate_condition(c),
            Condition::IsVisible(t) => self.store.get_indices(t).iter()
                .any(|&i| self.store.objects.get(i).map_or(false, |o| o.visible)),
            Condition::IsHidden(t)  => self.store.get_indices(t).iter()
                .any(|&i| self.store.objects.get(i).map_or(true,  |o| !o.visible)),
            Condition::Compare(left, op, right) => {
                match (
                    resolve_expr(left, &self.game_vars),
                    resolve_expr(right, &self.game_vars),
                ) {
                    (Some(l), Some(r)) => compare_operands(&l, &r, op).unwrap_or(false),
                    _ => false,
                }
            }
            Condition::VarExists(name) => self.game_vars.contains_key(name.as_str()),
            Condition::Grounded(target) => {
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx).map_or(false, |obj| obj.grounded)
                })
            }
            Condition::Expr(src) => {
                match parse_condition(src) {
                    Ok(condition) => self.evaluate_condition(&condition),
                    Err(e) => {
                        debug_assert!(false,
                            "[Condition::Expr] parse error in \"{src}\": {e}\n\
                            Use Condition::expr() to catch this at setup time.");
                        false
                    }
                }
            }
            Condition::HasTag(target, tag) => {
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx).map_or(false, |obj| obj.tags.contains(tag))
                })
            }

            // -- Crystalline physics conditions --
            Condition::IsSleeping(target) => {
                self.store.get_names(target).iter().any(|name| self.is_body_sleeping(name))
            }
            Condition::IsMoving(target) => {
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx).map_or(false, |obj| {
                        let speed_sq = obj.momentum.0 * obj.momentum.0 + obj.momentum.1 * obj.momentum.1;
                        speed_sq > 0.01
                    })
                })
            }
            Condition::IsRotating(target) => {
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx).map_or(false, |obj| {
                        obj.rotation_momentum.abs() > 0.01
                    })
                })
            }
            Condition::IsStill(target) => {
                self.store.get_indices(target).iter().all(|&idx| {
                    self.store.objects.get(idx).map_or(true, |obj| {
                        let speed_sq = obj.momentum.0 * obj.momentum.0 + obj.momentum.1 * obj.momentum.1;
                        speed_sq <= 0.01 && obj.rotation_momentum.abs() <= 0.01
                    })
                })
            }
            Condition::SpeedAbove(target, threshold) => {
                let t2 = threshold * threshold;
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx).map_or(false, |obj| {
                        obj.momentum.0 * obj.momentum.0 + obj.momentum.1 * obj.momentum.1 > t2
                    })
                })
            }
            Condition::SpeedBelow(target, threshold) => {
                let t2 = threshold * threshold;
                self.store.get_indices(target).iter().all(|&idx| {
                    self.store.objects.get(idx).map_or(true, |obj| {
                        obj.momentum.0 * obj.momentum.0 + obj.momentum.1 * obj.momentum.1 < t2
                    })
                })
            }
            Condition::CrystallineEnabled => self.crystalline.is_some(),
            Condition::EmitterActive(name) => {
                self.particle_system.as_ref().map_or(false, |ps| ps.has_emitter(name))
            }

            // -- Planet gravity conditions --
            Condition::OnPlanet(object_target, planet_target) => {
                let obj_indices    = self.store.get_indices(object_target);
                let planet_indices = self.store.get_indices(planet_target);

                obj_indices.iter().any(|&obj_idx| {
                    let obj = match self.store.objects.get(obj_idx) {
                        Some(o) => o,
                        None    => return false,
                    };
                    let obj_cx = obj.position.0 + obj.size.0 * 0.5;
                    let obj_cy = obj.position.1 + obj.size.1 * 0.5;

                    planet_indices.iter().any(|&planet_idx| {
                        let planet = match self.store.objects.get(planet_idx) {
                            Some(p) if p.planet_radius.is_some() => p,
                            _ => return false,
                        };
                        let radius    = planet.planet_radius.unwrap();
                        let planet_cx = planet.position.0 + planet.size.0 * 0.5;
                        let planet_cy = planet.position.1 + planet.size.1 * 0.5;

                        let dx = obj_cx - planet_cx;
                        let dy = obj_cy - planet_cy;
                        let dist = (dx * dx + dy * dy).sqrt();

                        dist <= radius + obj.size.0.max(obj.size.1) * 0.5 + 2.0
                    })
                })
            }

            Condition::InGravityField(object_target, planet_target) => {
                let obj_indices    = self.store.get_indices(object_target);
                let planet_indices = self.store.get_indices(planet_target);

                obj_indices.iter().any(|&obj_idx| {
                    let obj = match self.store.objects.get(obj_idx) {
                        Some(o) => o,
                        None    => return false,
                    };
                    let influence_mult = obj.gravity_influence_mult;
                    let obj_cx = obj.position.0 + obj.size.0 * 0.5;
                    let obj_cy = obj.position.1 + obj.size.1 * 0.5;

                    planet_indices.iter().any(|&planet_idx| {
                        let planet = match self.store.objects.get(planet_idx) {
                            Some(p) if p.planet_radius.is_some() => p,
                            _ => return false,
                        };
                        let radius    = planet.planet_radius.unwrap();
                        let planet_cx = planet.position.0 + planet.size.0 * 0.5;
                        let planet_cy = planet.position.1 + planet.size.1 * 0.5;

                        let dx = obj_cx - planet_cx;
                        let dy = obj_cy - planet_cy;
                        let dist_sq = dx * dx + dy * dy;
                        let influence = radius * influence_mult;
                        dist_sq <= influence * influence
                    })
                })
            }

            Condition::HasDominantPlanet(target) => {
                self.store.get_indices(target).iter().any(|&idx| {
                    self.store.objects.get(idx)
                        .map_or(false, |obj| obj.gravity_dominant_id.is_some())
                })
            }

            Condition::DominantPlanetIs(object_target, planet_target) => {
                let planet_names: Vec<String> = self.store.get_indices(planet_target)
                    .iter()
                    .filter_map(|&i| self.store.names.get(i).cloned())
                    .collect();

                self.store.get_indices(object_target).iter().any(|&idx| {
                    self.store.objects.get(idx)
                        .and_then(|obj| obj.gravity_dominant_id.as_deref())
                        .map_or(false, |dom| planet_names.iter().any(|n| n == dom))
                })
            }

            Condition::InAnyGravityField(object_target) => {
                let obj_indices = self.store.get_indices(object_target);
                obj_indices.iter().any(|&obj_idx| {
                    let obj = match self.store.objects.get(obj_idx) {
                        Some(o) => o,
                        None    => return false,
                    };
                    let influence_mult = obj.gravity_influence_mult;
                    let obj_cx = obj.position.0 + obj.size.0 * 0.5;
                    let obj_cy = obj.position.1 + obj.size.1 * 0.5;
                    self.store.objects.iter().any(|planet| {
                        let radius = match planet.planet_radius {
                            Some(r) => r,
                            None    => return false,
                        };
                        let planet_cx = planet.position.0 + planet.size.0 * 0.5;
                        let planet_cy = planet.position.1 + planet.size.1 * 0.5;
                        let dx = obj_cx - planet_cx;
                        let dy = obj_cy - planet_cy;
                        let dist_sq = dx * dx + dy * dy;
                        let field_r = radius * influence_mult;
                        dist_sq <= field_r * field_r
                    })
                })
            }

            // -- Grapple conditions --
            Condition::HasGrapple(target) => {
                self.store.get_names(target).iter().any(|name| self.has_grapple(name))
            }
            Condition::NoGrapple(target) => {
                self.store.get_names(target).iter().all(|name| !self.has_grapple(name))
            }
        }
    }

    // -- Typed game var accessors --

    pub fn set_var(&mut self, name: impl Into<String>, value: impl Into<Value>) {
        self.game_vars.insert(name.into(), value.into());
    }

    pub fn get_var(&self, name: &str) -> Option<Value> {
        self.game_vars.get(name).cloned()
    }

    pub fn has_var(&self, name: &str) -> bool {
        self.game_vars.contains_key(name)
    }

    pub fn remove_var(&mut self, name: &str) {
        self.game_vars.remove(name);
    }

    pub fn resolve(&self, expr: &Expr) -> Option<Value> {
        resolve_expr(expr, &self.game_vars)
    }

    pub fn modify_var(&mut self, name: &str, f: impl FnOnce(Value) -> Value) {
        if let Some(val) = self.game_vars.remove(name) {
            self.game_vars.insert(name.to_string(), f(val));
        }
    }

    pub fn get_u8(&self, name: &str) -> u8 {
        match self.game_vars.get(name) {
            Some(Value::U8(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected U8 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected U8 but key was missing"),
        }
    }

    pub fn get_u32(&self, name: &str) -> u32 {
        match self.game_vars.get(name) {
            Some(Value::U32(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected U32 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected U32 but key was missing"),
        }
    }

    pub fn get_i32(&self, name: &str) -> i32 {
        match self.game_vars.get(name) {
            Some(Value::I32(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected I32 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected I32 but key was missing"),
        }
    }

    pub fn get_f32(&self, name: &str) -> f32 {
        match self.game_vars.get(name) {
            Some(Value::F32(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected F32 but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected F32 but key was missing"),
        }
    }

    pub fn get_bool(&self, name: &str) -> bool {
        match self.game_vars.get(name) {
            Some(Value::Bool(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected Bool but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected Bool but key was missing"),
        }
    }

    pub fn get_usize(&self, name: &str) -> usize {
        match self.game_vars.get(name) {
            Some(Value::Usize(v)) => *v,
            Some(other) => panic!("game_var '{name}' expected Usize but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected Usize but key was missing"),
        }
    }

    pub fn get_str(&self, name: &str) -> &str {
        match self.game_vars.get(name) {
            Some(Value::Str(v)) => v.as_str(),
            Some(other) => panic!("game_var '{name}' expected Str but found {}", value_type_name(other)),
            None => panic!("game_var '{name}' expected Str but key was missing"),
        }
    }

    pub fn modify_u8(&mut self, name: &str, f: impl FnOnce(u8) -> u8) {
        match self.game_vars.get(name).cloned() {
            Some(Value::U8(n)) => { self.game_vars.insert(name.to_string(), Value::U8(f(n))); }
            Some(other) => panic!("game_var '{name}' expected U8 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected U8 for modify but key was missing"),
        }
    }

    pub fn modify_u32(&mut self, name: &str, f: impl FnOnce(u32) -> u32) {
        match self.game_vars.get(name).cloned() {
            Some(Value::U32(n)) => { self.game_vars.insert(name.to_string(), Value::U32(f(n))); }
            Some(other) => panic!("game_var '{name}' expected U32 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected U32 for modify but key was missing"),
        }
    }

    pub fn modify_i32(&mut self, name: &str, f: impl FnOnce(i32) -> i32) {
        match self.game_vars.get(name).cloned() {
            Some(Value::I32(n)) => { self.game_vars.insert(name.to_string(), Value::I32(f(n))); }
            Some(other) => panic!("game_var '{name}' expected I32 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected I32 for modify but key was missing"),
        }
    }

    pub fn modify_f32(&mut self, name: &str, f: impl FnOnce(f32) -> f32) {
        match self.game_vars.get(name).cloned() {
            Some(Value::F32(n)) => { self.game_vars.insert(name.to_string(), Value::F32(f(n))); }
            Some(other) => panic!("game_var '{name}' expected F32 for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected F32 for modify but key was missing"),
        }
    }

    pub fn modify_bool(&mut self, name: &str, f: impl FnOnce(bool) -> bool) {
        match self.game_vars.get(name).cloned() {
            Some(Value::Bool(n)) => { self.game_vars.insert(name.to_string(), Value::Bool(f(n))); }
            Some(other) => panic!("game_var '{name}' expected Bool for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected Bool for modify but key was missing"),
        }
    }

    pub fn modify_usize(&mut self, name: &str, f: impl FnOnce(usize) -> usize) {
        match self.game_vars.get(name).cloned() {
            Some(Value::Usize(n)) => { self.game_vars.insert(name.to_string(), Value::Usize(f(n))); }
            Some(other) => panic!("game_var '{name}' expected Usize for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected Usize for modify but key was missing"),
        }
    }

    pub fn modify_str(&mut self, name: &str, f: impl FnOnce(String) -> String) {
        match self.game_vars.get(name).cloned() {
            Some(Value::Str(s)) => { self.game_vars.insert(name.to_string(), Value::Str(f(s))); }
            Some(other) => panic!("game_var '{name}' expected Str for modify but found {}", value_type_name(&other)),
            None => panic!("game_var '{name}' expected Str for modify but key was missing"),
        }
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::I8(_)    => "I8",
        Value::U8(_)    => "U8",
        Value::I16(_)   => "I16",
        Value::U16(_)   => "U16",
        Value::I32(_)   => "I32",
        Value::U32(_)   => "U32",
        Value::I64(_)   => "I64",
        Value::U64(_)   => "U64",
        Value::F32(_)   => "F32",
        Value::F64(_)   => "F64",
        Value::Usize(_) => "Usize",
        Value::Bool(_)  => "Bool",
        Value::Str(_)   => "Str",
    }
}
