use crate::store::ObjectStore;
use crate::types::{Location, Anchor};

impl Location {
    pub(crate) fn resolve_position(&self, store: &ObjectStore) -> (f32, f32) {
        match self {
            Location::Position(pos) => *pos,
            Location::AtTarget(t) => {
                store.get_indices(t).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| o.position)
                    .unwrap_or((0.0, 0.0))
            }
            Location::Between(t1, t2) => {
                let p1 = store.get_indices(t1).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| o.position)
                    .unwrap_or((0.0, 0.0));
                let p2 = store.get_indices(t2).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| o.position)
                    .unwrap_or((0.0, 0.0));
                ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0)
            }
            Location::Relative { target, offset } => {
                store.get_indices(target).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| (o.position.0 + offset.0, o.position.1 + offset.1))
                    .unwrap_or(*offset)
            }
            Location::OnTarget { target, anchor, offset } => {
                store.get_indices(target).first()
                    .and_then(|&i| store.objects.get(i))
                    .map(|o| {
                        let ap = o.get_anchor_position(*anchor);
                        (ap.0 + offset.0, ap.1 + offset.1)
                    })
                    .unwrap_or(*offset)
            }
        }
    }
}