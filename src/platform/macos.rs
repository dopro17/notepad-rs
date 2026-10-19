use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::Result;
use muda::{AboutMetadata, PredefinedMenuItem, Submenu};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AllocAnyThread, class, define_class, msg_send, sel};
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSString, NSURL};

use crate::document::LineEnding;
use crate::menu::{Action, NativeMenu, native_item};

pub const DEFAULT_EOL: LineEnding = LineEnding::Lf;

pub const MONOSPACE_FONTS: &[(&str, u32)] = &[
    ("/System/Library/Fonts/Menlo.ttc", 0),
    ("/System/Library/Fonts/SFNSMono.ttf", 0),
];

fn app_menu() -> Result<Submenu> {
    let about = AboutMetadata {
        name: Some("notepad-rs".to_owned()),
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        comments: Some("Clone do Bloco de Notas em Rust".to_owned()),
        ..Default::default()
    };
    let menu = Submenu::new("notepad-rs", true);
    menu.append_items(&[
        &PredefinedMenuItem::about(Some("Sobre o notepad-rs"), Some(about)),
        &PredefinedMenuItem::separator(),
        &native_item(Action::Preferences),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::services(Some("Serviços")),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::hide(Some("Ocultar notepad-rs")),
        &PredefinedMenuItem::hide_others(Some("Ocultar outros")),
        &PredefinedMenuItem::show_all(Some("Mostrar tudo")),
        &PredefinedMenuItem::separator(),
        &native_item(Action::Quit),
    ])?;
    Ok(menu)
}

pub fn install_menu(cc: &eframe::CreationContext<'_>) -> Result<Option<NativeMenu>> {
    let _ = EGUI_CTX.set(cc.egui_ctx.clone());
    let native = NativeMenu::build(Some(app_menu()?), true)?;
    let window = Submenu::new("Janela", true);
    window.append_items(&[
        &PredefinedMenuItem::minimize(Some("Minimizar")),
        &PredefinedMenuItem::fullscreen(Some("Tela cheia")),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::bring_all_to_front(Some("Trazer tudo para a frente")),
    ])?;
    native.menu.append(&window)?;
    native.menu.init_for_nsapp();
    window.set_as_windows_menu_for_nsapp();
    Ok(Some(native))
}

// --------------------------------------------------------------------------
// "Abrir com" / duplo clique no Finder / arrastar para o ícone do Dock.
// O Finder envia o Apple Event kAEOpenDocuments em vez de argumentos de linha
// de comando; o winit não o trata, então registramos um handler próprio. Ele
// precisa ser instalado em applicationWillFinishLaunching: (depois dos handlers
// padrão do AppKit e antes de o evento de abertura ser entregue).

const K_CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
const K_AE_OPEN_DOCUMENTS: u32 = u32::from_be_bytes(*b"odoc");
const KEY_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static EGUI_CTX: OnceLock<egui::Context> = OnceLock::new();

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "NotepadRsOpenDocumentsHandler"]
    struct OpenDocumentsHandler;

    impl OpenDocumentsHandler {
        #[unsafe(method(willFinishLaunching:))]
        fn will_finish_launching(&self, _notification: &AnyObject) {
            let manager = NSAppleEventManager::sharedAppleEventManager();
            // SAFETY: o seletor existe neste objeto e tem a assinatura esperada.
            unsafe {
                let _: () = msg_send![
                    &*manager,
                    setEventHandler: self,
                    andSelector: sel!(handleOpen:withReply:),
                    forEventClass: K_CORE_EVENT_CLASS,
                    andEventID: K_AE_OPEN_DOCUMENTS
                ];
            }
        }

        #[unsafe(method(handleOpen:withReply:))]
        fn handle_open(&self, event: &NSAppleEventDescriptor, _reply: &NSAppleEventDescriptor) {
            let paths = file_paths(event);
            if let Ok(mut pending) = PENDING.lock() {
                pending.extend(paths);
            }
            if let Some(ctx) = EGUI_CTX.get() {
                ctx.request_repaint();
            }
        }
    }
);

fn file_paths(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
    // SAFETY: mensagens documentadas de NSAppleEventDescriptor com tipos corretos.
    unsafe {
        let direct: Option<Retained<NSAppleEventDescriptor>> =
            msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT];
        let Some(direct) = direct else {
            return Vec::new();
        };
        let count: isize = msg_send![&*direct, numberOfItems];
        let items: Vec<Retained<NSAppleEventDescriptor>> = if count == 0 {
            vec![direct]
        } else {
            (1..=count)
                .filter_map(|i| msg_send![&*direct, descriptorAtIndex: i])
                .collect()
        };
        items
            .iter()
            .filter_map(|item| {
                let url: Option<Retained<NSURL>> = msg_send![&**item, fileURLValue];
                Some(PathBuf::from(url?.path()?.to_string()))
            })
            .collect()
    }
}

/// Deve ser chamado antes de o loop de eventos iniciar.
pub fn early_init() {
    // SAFETY: init de NSObject; o handler vive até o fim do processo.
    let handler: Retained<OpenDocumentsHandler> =
        unsafe { msg_send![OpenDocumentsHandler::alloc(), init] };
    let name = NSString::from_str("NSApplicationWillFinishLaunchingNotification");
    // SAFETY: API documentada do NSNotificationCenter; o seletor existe no handler.
    unsafe {
        let center: Retained<AnyObject> = msg_send![class!(NSNotificationCenter), defaultCenter];
        let nil: Option<&AnyObject> = None;
        let _: () = msg_send![
            &*center,
            addObserver: &*handler,
            selector: sel!(willFinishLaunching:),
            name: &*name,
            object: nil
        ];
    }
    // Nem o NSNotificationCenter nem o NSAppleEventManager retêm o handler.
    std::mem::forget(handler);
}

pub fn take_open_requests() -> Vec<PathBuf> {
    PENDING
        .lock()
        .map(|mut p| std::mem::take(&mut *p))
        .unwrap_or_default()
}
