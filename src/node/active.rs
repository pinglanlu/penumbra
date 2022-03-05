use crate::{Elems, GetHash, Hash, Height, Inserted, Three};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Active<Sibling, Focus> {
    focus: Focus,
    siblings: Three<Sibling>,
    witnessed: bool,
    hash: Hash,
}

impl<Sibling, Focus> Active<Sibling, Focus> {
    pub(crate) fn from_parts(siblings: Three<Sibling>, focus: Focus) -> Self
    where
        Focus: crate::Active + GetHash,
        Sibling: Height + GetHash,
    {
        // Get the correct padding hash for this height
        let padding = Hash::padding();

        // Get the four elements of this segment, *in order*, and extract their hashes
        let (a, b, c, d) = match siblings.elems() {
            Elems::_0([]) => {
                let a = focus.hash();
                let [b, c, d] = [padding, padding, padding];
                (a, b, c, d)
            }
            Elems::_1(full) => {
                let [a] = full.map(Sibling::hash);
                let b = focus.hash();
                let [c, d] = [padding, padding];
                (a, b, c, d)
            }
            Elems::_2(full) => {
                let [a, b] = full.map(Sibling::hash);
                let c = focus.hash();
                let [d] = [padding];
                (a, b, c, d)
            }
            Elems::_3(full) => {
                let [a, b, c] = full.map(Sibling::hash);
                let d = focus.hash();
                (a, b, c, d)
            }
        };

        let hash = Hash::node(Focus::HEIGHT + 1, a, b, c, d);
        Self {
            witnessed: false,
            hash,
            siblings,
            focus,
        }
    }
}

impl<Sibling, Focus> Height for Active<Sibling, Focus>
where
    Sibling: Height,
    Focus: Height,
{
    const HEIGHT: usize = if Focus::HEIGHT == Sibling::HEIGHT {
        Focus::HEIGHT + 1
    } else {
        panic!("`Sibling::HEIGHT` does not match `Focus::HEIGHT` in `Segment`: check for improper depth types")
    };
}

impl<Sibling, Focus> GetHash for Active<Sibling, Focus> {
    fn hash(&self) -> Hash {
        self.hash
    }
}

impl<Sibling, Focus> crate::Active for Active<Sibling, Focus>
where
    Focus: crate::Active<Complete = Sibling> + GetHash,
    Sibling: crate::Complete<Active = Focus> + Height + GetHash,
{
    type Item = Focus::Item;
    type Complete = super::Complete<Sibling>;

    #[inline]
    fn singleton(item: Self::Item) -> Self {
        let focus = Focus::singleton(item);
        let siblings = Three::new();
        Self::from_parts(siblings, focus)
    }

    #[inline]
    fn witness(&mut self) {
        if !self.witnessed {
            self.witnessed = true;
            self.focus.witness();
        }
    }

    #[inline]
    fn complete(self) -> Self::Complete {
        super::Complete::from_siblings_and_focus_unchecked(
            self.hash,
            self.siblings,
            self.focus.complete(),
        )
    }

    #[inline]
    fn insert(self, item: Self::Item) -> Inserted<Self> {
        let Active {
            witnessed,
            focus,
            siblings,
            hash, // NOTE: ONLY VALID TO RE-USE WHEN CONSTRUCTING A NODE
        } = self;

        match focus.insert(item) {
            // We successfully inserted at the focus, so siblings don't need to be changed
            Inserted::Success(focus) => Inserted::Success(Self::from_parts(siblings, focus)),

            // We failed to insert at the focus and we should not carry on to keep inserting at a
            // new focus, so we propagate the error
            Inserted::Failure(item, focus) => Inserted::Failure(
                item,
                Self {
                    witnessed,
                    focus,
                    siblings,
                    hash,
                },
            ),

            // We couldn't insert at the focus because it was full, so we need to move our path
            // rightwards and insert into a newly created focus
            Inserted::Full(item, sibling) => match siblings.push(sibling) {
                // We were instructed to carry the item to a fresh active focus, and we had enough
                // room to add another sibling, so we set our focus to a new focus containing only
                // the item we couldn't previously insert
                Ok(siblings) => {
                    let focus = Focus::singleton(item);
                    Inserted::Success(Self::from_parts(siblings, focus))
                }
                // We didn't have enough room to add another sibling, so we return a complete node
                // as a carry, to be propagated up above us and added to some ancestor segment's
                // siblings, along with the item we couldn't insert
                Err(complete) => {
                    let node = super::Complete::from_parts_unchecked(
                        // We can avoid recomputing this hash because our hash calculation is
                        // carefully designed to hash in the exact same order as the hash
                        // calculation for a node itself
                        hash,
                        // If this segment was not marked as witnessed, we know that any
                        // sub-segments are likewise not witnessed, so we can erase the subtree
                        complete,
                    );
                    Inserted::Full(item, node)
                }
            },
        }
    }
}
