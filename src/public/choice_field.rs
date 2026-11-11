//! `Aspose.Pdf.Forms.ChoiceField` — the intermediate base for
//! choice fields (combo / list / radio), per
//! `capabilities/choice_field.yaml`.
//!
//! Modelled as the [`ChoiceField`] trait (extending
//! [`Field`]) over a shared `ChoiceBase`.

use crate::public::field::Field;
use crate::public::option::Option as FormOption;
use crate::public::option_collection::OptionCollection;

/// Shared choice-field fields. Crate-internal.
/// The `/Ff` bits a choice field takes from its shared base
/// (§12.7.4 Table 230): MultiSelect 22 and CommitOnSelChange 27.
pub(crate) fn choice_flags(choice: &ChoiceBase) -> i64 {
    let mut f = 0;
    if choice.multi_select {
        f |= 1 << 21;
    }
    if choice.commit_immediately {
        f |= 1 << 26;
    }
    f
}

/// The dictionary entries a choice field takes from its shared base
/// (§12.7.4.4): `/Opt` is the option list a viewer offers — without it
/// a drop-down or list box reaches the file with nothing to choose
/// from — and `/I` the indices currently selected, whether they came
/// from `SelectedItems` or from the single `Selected`.
///
/// An option whose display name differs from its export value is
/// written as the two-string array the spec gives for that case, so a
/// form's export keeps the value while the user sees the name.
pub(crate) fn choice_entries(
    choice: &ChoiceBase,
) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
    use crate::internal::annotation_io::Entry;
    let mut out = Vec::new();
    if choice.options.count() > 0 {
        let items = (0..choice.options.count())
            .filter_map(|i| choice.options.at(i))
            .map(|o| {
                if o.name().is_empty() || o.name() == o.value() {
                    Entry::Text(o.value().to_string())
                } else {
                    Entry::Array(vec![
                        Entry::Text(o.value().to_string()),
                        Entry::Text(o.name().to_string()),
                    ])
                }
            })
            .collect();
        out.push(("Opt", Entry::Array(items)));
    }
    // The plural property wins when both are set: it is the explicit
    // statement, and `Selected` is the one-option case of it.
    let single = (choice.selected >= 0
        && (choice.selected as usize) < choice.options.count())
        .then_some(vec![choice.selected]);
    if let Some(mut indices) =
        (!choice.selected_items.is_empty()).then(|| choice.selected_items.clone()).or(single)
    {
        // §12.7.4.4: the indices are written in ascending order.
        indices.sort_unstable();
        out.push((
            "I",
            Entry::Array(indices.into_iter().map(|i| Entry::Int(i as i64)).collect()),
        ));
    }
    out
}

pub(crate) struct ChoiceBase {
    pub(crate) options: OptionCollection,
    pub(crate) commit_immediately: bool,
    pub(crate) multi_select: bool,
    pub(crate) selected: i32,
    pub(crate) selected_items: Vec<i32>,
}

impl ChoiceBase {
    pub(crate) fn new() -> Self {
        Self {
            options: OptionCollection::new(),
            commit_immediately: false,
            multi_select: false,
            selected: -1,
            selected_items: Vec::new(),
        }
    }
}

impl Default for ChoiceBase {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) mod sealed {
    use super::ChoiceBase;

    /// Choice-base access for concrete choice fields.
    pub(crate) trait HasChoiceBase {
        /// The shared choice base.
        fn choice(&self) -> &ChoiceBase;
        /// The shared choice base, mutably.
        fn choice_mut(&mut self) -> &mut ChoiceBase;
    }
}

/// A choice field offering a list of options.
#[allow(private_bounds)]
pub trait ChoiceField: Field + sealed::HasChoiceBase {
    /// Add an option by export value.
    fn add_option(&mut self, export_value: String) {
        let mut o = FormOption::new();
        o.set_value(export_value.clone());
        o.set_name(export_value);
        self.choice_mut().options.add(o);
    }

    /// Add an option with an explicit export value and display name.
    fn add_option_named(&mut self, export_value: String, name: String) {
        let mut o = FormOption::new();
        o.set_value(export_value);
        o.set_name(name);
        self.choice_mut().options.add(o);
    }

    /// Remove the option with the given display name.
    fn delete_option(&mut self, name: &str) {
        if let std::option::Option::Some(o) =
            self.choice().options.by_name(name).cloned()
        {
            self.choice_mut().options.remove(&o);
        }
    }

    /// Whether edits commit immediately.
    fn commit_immediately(&self) -> bool {
        self.choice().commit_immediately
    }
    /// Set whether edits commit immediately.
    fn set_commit_immediately(&mut self, value: bool) {
        self.choice_mut().commit_immediately = value;
    }

    /// Whether multiple options may be selected.
    fn multi_select(&self) -> bool {
        self.choice().multi_select
    }
    /// Set whether multiple options may be selected.
    fn set_multi_select(&mut self, value: bool) {
        self.choice_mut().multi_select = value;
    }

    /// The zero-based index of the selected option (`-1` when none) —
    /// the same index space as
    /// [`selected_items`](ChoiceField::selected_items) and as the `/I`
    /// array §12.7.4.4 writes them both into.
    fn selected(&self) -> i32 {
        self.choice().selected
    }
    /// Select the option at `index`.
    ///
    /// §12.7.4.4 gives a choice field two entries for a selection: `/V`
    /// is the value a viewer displays and `/I` the indices it was
    /// chosen by. So selecting an option also sets the field
    /// [`value`](Field::value) to that option's export value — an index
    /// alone would record a choice the viewer would not show. An index
    /// outside the option list sets neither.
    fn set_selected(&mut self, index: i32) {
        self.choice_mut().selected = index;
        if index < 0 {
            return;
        }
        if let std::option::Option::Some(option) =
            self.choice().options.at(index as usize).cloned()
        {
            self.set_value(option.value().to_string());
        }
    }

    /// The zero-based indices of all selected options, as written into
    /// `/I`.
    fn selected_items(&self) -> &[i32] {
        &self.choice().selected_items
    }
    /// Set the indices of all selected options.
    fn set_selected_items(&mut self, value: Vec<i32>) {
        self.choice_mut().selected_items = value;
    }

    /// The field's options.
    fn options(&self) -> &OptionCollection {
        &self.choice().options
    }
}
