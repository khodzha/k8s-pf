use cpp_core::{Ptr, StaticUpcast};
use qt_core::{
    q_init_resource, qs, AlignmentFlag, CheckState, ItemDataRole, QBox, QObject, QPtr,
    QSortFilterProxyModel, QStringList,
};
use qt_gui::{QStandardItem, QStandardItemModel};
use qt_ui_tools::ui_form;
use qt_widgets::{q_header_view::ResizeMode, QApplication, QTableView, QWidget};
use std::rc::Rc;

#[ui_form("../ui/form.ui")]
#[derive(Debug)]
struct Form {
    widget: QBox<QWidget>,
    table: QPtr<QTableView>,
}

#[derive(Debug)]
struct TodoWidget {
    form: Form,
    source_model: QBox<QStandardItemModel>,
    proxy_model: QBox<QSortFilterProxyModel>,
}

impl StaticUpcast<QObject> for TodoWidget {
    unsafe fn static_upcast(ptr: Ptr<Self>) -> Ptr<QObject> {
        ptr.form.widget.as_ptr().static_upcast()
    }
}

impl TodoWidget {
    fn new() -> Rc<Self> {
        unsafe {
            let source_model = QStandardItemModel::new_0a();
            source_model.set_column_count(3);
            let labels = QStringList::new();

            labels.push_back(&qs(""));
            labels.push_back(&qs("Name"));
            labels.push_back(&qs("Port"));

            source_model.set_horizontal_header_labels(&labels);

            let form = Form::load();

            form.table
                .horizontal_header()
                .set_section_resize_mode_1a(ResizeMode::Stretch);
            form.table
                .horizontal_header()
                .set_section_resize_mode_2a(0, ResizeMode::ResizeToContents);

            let this = Rc::new(TodoWidget {
                form,
                source_model,
                proxy_model: QSortFilterProxyModel::new_0a(),
            });
            this.init();
            this
        }
    }

    unsafe fn init(self: &Rc<Self>) {
        for (idx, text) in ["Learn Qt", "Learn Rust", "Conquer the world"]
            .into_iter()
            .enumerate()
        {
            let checkable = QStandardItem::new().into_ptr();
            checkable.set_checkable(true);
            checkable.set_text(&qs("q"));
            checkable.set_check_state(CheckState::Unchecked);
            checkable.set_text_alignment(AlignmentFlag::AlignCenter.into());
            self.source_model.append_row_q_standard_item(checkable);
            self.source_model.set_item_3a(idx as i32, 0, checkable);

            let item = QStandardItem::new().into_ptr();
            item.set_text(&qs(text));
            self.source_model.set_item_3a(idx as i32, 1, item);

            //            self.source_model.append_row_q_standard_item(item);

            let port = QStandardItem::new().into_ptr();
            port.set_text(&qs("5432"));
            port.set_text_alignment(AlignmentFlag::AlignCenter.into());
            self.source_model.set_item_3a(idx as i32, 2, port);

            let port = QStandardItem::new().into_ptr();
            port.set_text_alignment(AlignmentFlag::AlignCenter.into());
            self.source_model.set_item_3a(idx as i32, 3, port);
        }

        self.proxy_model.set_source_model(&self.source_model);
        self.proxy_model
            .set_filter_role(ItemDataRole::CheckStateRole.into());

        self.form.table.set_model(&self.proxy_model);
    }

    fn show(self: &Rc<Self>) {
        unsafe {
            self.form.widget.show();
        }
    }
}

pub fn qrun() -> ! {
    let (jh, shutdown) = crate::k8s::start();

    QApplication::init(|_| {
        q_init_resource!("resources");
        let todo_widget = TodoWidget::new();
        todo_widget.show();
        let result = unsafe { QApplication::exec() };

        if shutdown.send(true).is_err() {
            tracing::warn!(
                "K8s loop was already dropped by the time we sent shutdown notification"
            );
        }

        if let Err(e) = jh.join() {
            tracing::error!(error = ?e, "K8s handler failed");
        }

        result
    })
}
