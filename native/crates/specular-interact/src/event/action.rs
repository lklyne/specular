//! What a menu, a key binding, a control or an agent asks the app to do.

use specular_core::Camera;
use specular_doc::{AnnotationId, EntityId, ItemId};

use crate::{
    ArrangeMode, CanvasId, ChatAction, Format, PageStep, Property, RepoAction, SettingAction,
    SidebarAction, SpaceAction, Tool, ToolDefaultPatch,
};

/// A command with no pointer position: what a key binding, a menu item, a
/// toolbar button or an API route asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Escape, one stage a press. A comment draft is dropped; with none, a
    /// focused comment loses the focus. Otherwise: abandon the gesture in
    /// flight, return to the select tool and leave the entered page, and
    /// with none of those to back out of, clear the selection.
    Cancel,
    /// Switch tool.
    SetTool(Tool),
    /// Choose the theme and save the choice.
    SetTheme(crate::Theme),
    /// Change one tool default and save the defaults.
    SetToolDefault(ToolDefaultPatch),
    /// Switch to the tool a default belongs to and change that default: what
    /// a variant key such as Shift+R or Shift+M does.
    SetToolVariant(ToolDefaultPatch),
    /// Undo the latest document step. While text is being edited, the
    /// latest step of that edit instead.
    Undo,
    /// Redo the latest undone step, of the document or of the edit.
    Redo,
    /// An Edit menu command done inside the entered page, which is what the
    /// menu's items are while a page has the keys. Nothing without one.
    PageEdit(specular_core::PageEdit),
    /// Replace the selection. Ids that name nothing are dropped.
    Select(Vec<ItemId>),
    /// Select `select`, as [`Action::Select`] does, and bring `focus` into
    /// view: nothing moves when it is all in the part of the viewport the
    /// sidebar leaves free, and otherwise the camera pans, at the zoom it
    /// has, to centre it there. What a sidebar row sends.
    Reveal {
        /// What the selection becomes.
        select: Vec<ItemId>,
        /// What the camera shows.
        focus: ItemId,
    },
    /// Give a comment the focus, as [`Action::FocusComment`] does, and bring
    /// what it is on into view as [`Action::Reveal`] does.
    RevealComment(AnnotationId),
    /// Show or hide the sidebar, fold a section or open a row.
    Sidebar(SidebarAction),
    /// Move the camera.
    SetCamera(Camera),
    /// Remove the selection, with what is inside its groups, what is hooked
    /// to its pages and the edges that would lose an end.
    Delete,
    /// Copy the selection into free space beside it and select the copies.
    Duplicate,
    /// Put the selection on the clipboard.
    Copy,
    /// Put the selection on the clipboard and remove it.
    Cut,
    /// Paste the clipboard at the pointer.
    Paste,
    /// Select everything that is not inside a group.
    SelectAll,
    /// Zoom in one step about the middle of the viewport.
    ZoomIn,
    /// Zoom out one step about the middle of the viewport.
    ZoomOut,
    /// Zoom to 100% about the middle of the viewport.
    ZoomReset,
    /// Zoom to this many percent about the middle of the viewport. It names
    /// the level and not the camera it lands on, so the control that holds
    /// it is the same wherever the canvas is panned.
    ZoomTo(u16),
    /// Show everything on the canvas, centred.
    ZoomToFit,
    /// Toggle markdown formatting on the selection of the text being
    /// edited. Does nothing where the text does not take that format.
    Format(Format),
    /// Move the selection one slot forward in the stack order.
    BringForward,
    /// Move the selection one slot backward in the stack order.
    SendBackward,
    /// Move the selection in front of everything.
    BringToFront,
    /// Move the selection behind everything.
    SendToBack,
    /// Wrap the selected items in a new group and select it.
    Group,
    /// Take the selected group apart and select what was inside it.
    Ungroup,
    /// Open a comment draft on the region the selected entities span.
    AnnotateSelection,
    /// Lay the selected items out in a row, a column or a grid, keeping the
    /// footprint they have and evening the spacing inside it. Does nothing
    /// for fewer than two.
    Arrange(ArrangeMode),
    /// Make the selection an auto-layout group (ADR 0015) and select it: a
    /// lone group manages its members as a row or a column, whichever they
    /// are spread along, and two or more items are grouped first.
    AutoLayout,
    /// Pack the selected group's members along an axis, or with `None` let
    /// them sit where they are.
    GroupLayout(Option<crate::LayoutAxis>),
    /// Set the gap the selected auto-layout group packs with, in canvas
    /// units.
    GroupGap(f64),
    /// Show the canvas, or one page or Document in its tab. View state:
    /// nothing is written to the document and no undo step is made.
    Show(crate::Showing),
    /// Open this page or Document in a tab from the canvas. A tab it does
    /// not have yet starts in Device; one it has keeps its lens. View
    /// state, like [`Self::Show`].
    OpenTab(EntityId),
    /// A press on the tab of this item, before the click that shows it:
    /// a page is laid out at the size its tab will give it, and what is
    /// drawn stays as it is. `None` is the press ending without a click,
    /// which puts the page back. View state, like [`Self::Show`].
    PrepareShow(Option<EntityId>),
    /// Look at the item of the tab showing through this lens. View state,
    /// kept by the tab for the session.
    SetLens(crate::Lens),
    /// Open or shut the eye: whether Device draws what is around its item.
    /// View state, one choice for every tab.
    ShowOthers(bool),
    /// Show the tab after the one showing, going round to the first.
    ShowNext,
    /// Show the tab before the one showing, going round to the last.
    ShowPrevious,
    /// Close the tab of this item, or with `None` the tab showing. The
    /// item stays on the canvas. Closing the tab showing shows its
    /// neighbour, or the canvas after the last one; closing another leaves
    /// what is shown. View state, like [`Self::Show`].
    CloseTab(Option<EntityId>),
    /// Make a page at the page tool's preset in a free spot of the canvas,
    /// show its tab and put the caret in its address. The page is one undo
    /// step.
    NewPageTab,
    /// Put the caret in the address of the page whose controls the dock
    /// holds.
    EditPageUrl,
    /// Give a comment the focus, taking the selection away, or with `None`
    /// let go of the focus. An id that is not shown does nothing.
    FocusComment(Option<AnnotationId>),
    /// Mark a comment resolved. With `None`, every comment on the focused
    /// comment's mark, as one step. Does nothing with no such comment.
    ResolveComment(Option<AnnotationId>),
    /// Remove a comment. With `None`, every comment on the focused
    /// comment's mark, as one step. Does nothing with no such comment.
    DeleteComment(Option<AnnotationId>),
    /// Take the entered page, or the one selected page, an entry back in
    /// its history. Does nothing where there is none.
    PageBack,
    /// Take that page an entry forward in its history.
    PageForward,
    /// Load that page's address again.
    PageReload,
    /// Abandon that page's load in flight.
    PageStop,
    /// Take that page to an address, as one typed in its address field.
    PageNavigate(String),
    /// Move a setting of the selected pages to its next value: the frame
    /// on or off, the orientation turned, the color scheme one on.
    StepPage(PageStep),
    /// Toggle the sync set of the selected pages (ADR 0027). Two or more
    /// pages that already share one set leave it; otherwise they all join
    /// one new set. One selected page that is in a set leaves it. A set
    /// left with one page is dissolved. One undo step.
    ToggleSync,
    /// Change the space's canvases: show another, add, rename, copy or
    /// remove one.
    Canvas(CanvasAction),
    /// Something done in the right panel: its threads, its composer.
    Chat(ChatAction),
    /// A change to the connected repos and the origins bound to them.
    Repo(RepoAction),
    /// Choose, open or reveal the space folder, or quit from the first-run
    /// view.
    Space(SpaceAction),
    /// A change to what is kept between launches.
    Setting(SettingAction),
    /// Set one field of the selection, as a dock control does. It applies
    /// to every selected item it means something for, as one undo step.
    SetProperty(Property),
    /// Move the selection by exactly this many canvas units.
    Nudge {
        /// Along x. Positive is right.
        dx: f64,
        /// Along y. Positive is down.
        dy: f64,
    },
}

/// Something done to the canvases of the space.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    /// Show another canvas. The one being left keeps its document, its
    /// undo history, its camera and its selection.
    Switch(CanvasId),
    /// Add an empty canvas named `Canvas N` and show it.
    New,
    /// Give a canvas another name, and its file with it. `None` is the
    /// active canvas. Does nothing when the name is empty or taken.
    Rename {
        /// The canvas.
        canvas: Option<CanvasId>,
        /// The new name.
        name: String,
    },
    /// Start typing a new name for a canvas in its row of the sidebar. `None`
    /// is the active canvas. Does nothing while the sidebar is hidden.
    BeginRename(Option<CanvasId>),
    /// Copy a canvas into a new one beside it and show the copy. `None` is
    /// the active canvas.
    Duplicate(Option<CanvasId>),
    /// Remove a canvas and send its file to the trash. `None` is the
    /// active canvas. The space's last canvas is replaced by an empty one.
    Delete(Option<CanvasId>),
}
