//! A virtualized list component for Yew.

#![deny(
    missing_docs,
    missing_debug_implementations,
    bare_trait_objects,
    anonymous_parameters,
    elided_lifetimes_in_paths
)]

use core::fmt;
use std::fmt::Display;
use std::rc::Rc;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::Element;
use yew::html::IntoPropValue;
use yew::prelude::*;

/// A wrapper around a method generating individual items in the list.
///
/// To construct such a generator, use [`VirtualList::item_gen`]
pub struct ItemGenerator {
    gen: Rc<dyn Fn(usize) -> Html>,
}

impl fmt::Debug for ItemGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemGenerator")
            .field("gen", &"<function ptr>")
            .finish_non_exhaustive()
    }
}

impl ItemGenerator {
    fn emit(&self, idx: usize) -> Html { (self.gen)(idx) }
}

impl PartialEq for ItemGenerator {
    #[allow(clippy::vtable_address_comparisons)] // We don't care about false negatives
    fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.gen, &other.gen) }
}

impl VirtualList {
    /// Construct an [`ItemGenerator`] that can be passed as a value of
    /// [`VirtualListProps::items`].
    pub fn item_gen(gen: impl 'static + Fn(usize) -> Html) -> ItemGenerator { ItemGenerator { gen: Rc::new(gen) } }
}

/// The height of each items, usually given in pixels.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ItemSize {
    /// A height in pixels
    Pixels(usize),
}

impl ItemSize {
    fn as_scroll_size(&self) -> i32 {
        match self {
            Self::Pixels(pxs) => (*pxs).try_into().unwrap(),
        }
    }
}

impl IntoPropValue<ItemSize> for usize {
    fn into_prop_value(self) -> ItemSize { ItemSize::Pixels(self) }
}

impl Display for ItemSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pixels(pxs) => write!(f, "{pxs}px"),
        }
    }
}

impl std::ops::Mul<&'_ ItemSize> for usize {
    type Output = ItemSize;

    fn mul(self, rhs: &ItemSize) -> Self::Output {
        match rhs {
            ItemSize::Pixels(pxs) => ItemSize::Pixels(self * pxs),
        }
    }
}

#[wasm_bindgen]
extern "C" {
    type PositionedElementDuck;
    #[wasm_bindgen(method, getter, structural, js_name = __yew_resize_obs_pos)]
    fn pos(this: &PositionedElementDuck) -> usize;
    #[wasm_bindgen(method, setter, structural, js_name = __yew_resize_obs_pos)]
    fn set_pos(this: &PositionedElementDuck, pos: usize);
}

#[derive(Properties)]
struct ScrollWrapperProps {
    children: Children,
    classes: Classes,
    style:Option<String>
}

impl PartialEq for ScrollWrapperProps {
    fn eq(&self, other: &Self) -> bool { self.children == other.children }
}

#[function_component(ScrollItemWrapper)]
fn scroll_item_wrapper(props: &ScrollWrapperProps) -> Html {
    let wrapped_ref = use_node_ref();
    html! {
        <div ref={&wrapped_ref} class={props.classes.clone()} style={props.style.clone()}>
            {props.children.clone()}
        </div>
    }
}

/// Backing scroll state, as source of truth for item sizes, etc.
#[derive(Debug)]
struct BackingScrollState {
}

#[derive(Debug)]
struct ScrollManager {
    shared: Rc<BackingScrollState>,
}

impl ScrollManager {
    fn new() -> Self {
        let shared = {
            Rc::new(BackingScrollState {
            })
        };
        ScrollManager {
            shared,
        }
    }

    fn mounted(&mut self, host: Element) {
        // TODO: TRIGGER SCROLL EVENT HERE
    }

    // Scroll pin to bottom logic comes from https://css-tricks.com/books/greatest-css-tricks/pin-scrolling-to-bottom/
    fn generate_contents(&self, props: &VirtualListProps) -> Html {
        let autoscroll_latch = matches!(props.autoscroll, AutoscrollMode::BottomLatch);
        let after_plus = if autoscroll_latch { 1 } else { 0 };
        let (regularStyleExtra, postStyleExtra) = if autoscroll_latch {
            (
                Some("overflow-anchor: none;"),
                Some("overflow-anchor: auto;")
            )
        } else {
            (
                None,
                None
            )
        };

        let items = (0..props.item_count).map(|i| {
            let item = props.items.emit(i);
            html! {
                <ScrollItemWrapper key={i} classes={props.item_classes.clone()} style={regularStyleExtra}>
                    {item}
                </ScrollItemWrapper>
            }
        });

        html! {
            <>
            <div key="wrap" style={"display: contents"}>
            {for items}
            <div key="post" style={format!("height: {after_plus}px;{}", postStyleExtra.unwrap_or(""))} />
            </div>
            </>
        }
    }
}

/// Options for autoscroll property
#[derive(Default, Debug, PartialEq)]
pub enum AutoscrollMode {
    /// No special behavior.
    #[default]
    None,
    /// When scrolled to bottom, new content causes scroll to stay at bottom
    BottomLatch,
}

/// Properties for a [`VirtualList`].
#[derive(PartialEq, Properties, Debug)]
pub struct VirtualListProps {
    /// A callback to render individual items. Only invoked for items on screen.
    /// Use [`VirtualList::item_gen`] to create an [`ItemGenerator`].
    pub items: ItemGenerator,
    /// The number of items in the list, in total. Items that are not visible on
    /// screen take up scroll space and are lazily instantiated when the user
    /// scrolls to them later.
    pub item_count: usize,
    /// An approximate height for items that haven't been rendered, yet, but
    /// should still take up scroll space. After the first render of an
    /// item, the height will be adjusted automatically and measured.
    ///
    /// Setting this to an inaccurate value will mis-represent the remaining
    /// scroll distance, but cause no other ill effects.
    pub height_prior: ItemSize,
    /// If set, the div will scroll automatically when the list length changes
    #[prop_or_default]
    pub autoscroll: AutoscrollMode,

    /// Additional classes to apply to the scroll list itself.
    ///
    /// ### Gotcha
    ///
    /// The list itself is rendered without a max height or other layout
    /// constraints to stay independent of a particular css solution. Use these
    /// additional classes to apply additional css to the list.
    pub classes: Classes,
    /// Individual items are wrapped in a `<div>` to take their measurements in
    /// a block context. The classes here are applied to each such wrapper.
    /// Usually, you don't need to supply this property.
    #[prop_or_default]
    pub item_classes: Classes,
}

/// Internal message type for a [`VirtualList`].
#[derive(Debug)]
pub struct VirtualListMsg(ScrollMsg);

#[derive(Debug)]
enum ScrollMsg {
    None
}

/// A virtualized list, rendering only items that are also shown on screen to
/// the user.
///
/// ## Example
///
/// ```
/// use yew::prelude::*;
/// use yew_virtualized::VirtualList;
///
/// fn items(idx: usize) -> Html {
///     html! { format!("Item #{idx}") }
/// }
///
/// #[function_component(App)]
/// fn app() -> Html {
///     html! {
///         <VirtualList
///             item_count={100}
///             height_prior={30}
///             items={VirtualList::item_gen(items)}
///             classes={"scrollbar"} />
///     }
/// }
/// ```
#[derive(Debug)]
pub struct VirtualList {
    manager: ScrollManager,
    host_ref: NodeRef,
}

impl Component for VirtualList {
    type Message = VirtualListMsg;
    type Properties = VirtualListProps;

    fn create(ctx: &Context<Self>) -> Self {
        let manager = ScrollManager::new();
        let host_ref = NodeRef::default();
        Self {
            manager,
            host_ref,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        false // No messages used in this component
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        let contents = self.manager.generate_contents(props);

        html! {
            <div ref={&self.host_ref} class={props.classes.clone()} style="overflow-y: scroll;">
                {contents}
            </div>
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _props: &<Self as yew::Component>::Properties) -> bool {
        true // Re-render on change
    }

    fn rendered(&mut self, _: &Context<Self>, first_render: bool) {
        if first_render {
            let host = self.host_ref.cast::<Element>().unwrap();
            self.manager.mounted(host);
        }
    }
}
