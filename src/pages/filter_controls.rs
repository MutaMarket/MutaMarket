//! Small, reusable filter controls for the module browser sidebar.
//!
//! Each control renders its element **once** and drives its active look from
//! a reactive slice signal (the `Button`'s `variant`, the `Checkbox`'s
//! `checked`). Because nothing is rebuilt inside a `{move || ...}` closure,
//! changing a filter updates only that control's classes in place - no DOM
//! recreation, so the control never flickers. The pure decision logic lives
//! in free functions so it can be unit-tested without a reactive runtime.

use leptos::prelude::*;

use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::checkbox::Checkbox;

/// Whether a sort field is active, and if so its direction, for a given sort
/// state. `descending` is the `bool` in the `(field, descending)` pair.
pub fn sort_direction(sort: &Option<(String, bool)>, field: &str) -> Option<bool> {
    sort.as_ref()
        .filter(|(current, _)| current == field)
        .map(|(_, descending)| *descending)
}

/// The legacy sort cycle for one field: off, then ascending, then descending,
/// then off again.
pub fn cycle_sort(current: Option<bool>) -> Option<bool> {
    match current {
        None => Some(false),
        Some(false) => Some(true),
        Some(true) => None,
    }
}

/// The arrow suffix shown next to an active sort label.
fn sort_arrow(direction: Option<bool>) -> &'static str {
    match direction {
        Some(false) => " \u{2191}",
        Some(true) => " \u{2193}",
        None => "",
    }
}

/// One sort toggle. Clicking cycles the field through
/// off -> ascending -> descending and reports the new sort pair (or `None`).
#[component]
pub fn SortButton(
    field: &'static str,
    label: &'static str,
    #[prop(into)] sort: Signal<Option<(String, bool)>>,
    on_change: Callback<Option<(String, bool)>>,
) -> impl IntoView {
    let direction = Memo::new(move |_| sort_direction(&sort.get(), field));
    let variant = Signal::derive(move || match direction.get() {
        Some(_) => ButtonVariant::Default,
        None => ButtonVariant::Outline,
    });

    view! {
        <Button
            variant=variant
            size=ButtonSize::Sm
            class="h-7 px-2 text-xs"
            on:click=move |_| {
                let next = cycle_sort(direction.get_untracked())
                    .map(|descending| (field.to_owned(), descending));
                on_change.run(next);
            }
        >
            {label}
            {move || sort_arrow(direction.get())}
        </Button>
    }
}

/// One contract-type choice (or the "Any" reset when `value` is `None`).
/// Highlighted while it matches the selected contract type.
#[component]
pub fn ContractTypeButton(
    label: &'static str,
    #[prop(optional)] value: Option<&'static str>,
    #[prop(into)] selected: Signal<Option<String>>,
    on_select: Callback<Option<String>>,
) -> impl IntoView {
    let active = Memo::new(move |_| selected.get().as_deref() == value);
    let variant = Signal::derive(move || {
        if active.get() { ButtonVariant::Default } else { ButtonVariant::Outline }
    });

    view! {
        <Button
            variant=variant
            size=ButtonSize::Sm
            class="h-7 px-2 text-xs"
            on:click=move |_| on_select.run(value.map(str::to_owned))
        >
            {label}
        </Button>
    }
}

/// One boolean filter flag. The checkbox reflects `checked` reactively and
/// reports the new value on toggle.
#[component]
pub fn FilterCheckbox(
    label: &'static str,
    #[prop(into)] checked: Signal<bool>,
    on_toggle: Callback<bool>,
) -> impl IntoView {
    view! {
        <label class="flex items-center gap-2 text-xs text-muted-foreground">
            <Checkbox checked=checked aria_label=label on_checked_change=on_toggle/>
            {label}
        </label>
    }
}

#[cfg(test)]
mod tests {
    use super::{cycle_sort, sort_direction};

    #[test]
    fn cycle_sort_walks_off_ascending_descending_off() {
        assert_eq!(cycle_sort(None), Some(false));
        assert_eq!(cycle_sort(Some(false)), Some(true));
        assert_eq!(cycle_sort(Some(true)), None);
    }

    #[test]
    fn sort_direction_reads_only_the_matching_field() {
        let sort = Some(("price".to_owned(), true));
        assert_eq!(sort_direction(&sort, "price"), Some(true));
        assert_eq!(sort_direction(&sort, "value"), None);
        assert_eq!(sort_direction(&None, "price"), None);
    }
}
