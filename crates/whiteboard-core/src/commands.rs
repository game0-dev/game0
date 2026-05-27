use crate::document::WhiteboardDoc;
use crate::object::{ObjectId, WhiteboardObject};

#[derive(Debug, Clone)]
pub enum Command {
    CreateObject { object: WhiteboardObject },
    DeleteObject { object: WhiteboardObject },
    MoveObjects { ids: Vec<ObjectId>, dx: f32, dy: f32 },
    UpdateText { id: ObjectId, before: String, after: String },
}

pub fn apply_command(doc: &mut WhiteboardDoc, command: &Command) -> Option<Command> {
    match command {
        Command::CreateObject { object } => {
            doc.insert_object(object.clone());
            Some(Command::DeleteObject {
                object: object.clone(),
            })
        }
        Command::DeleteObject { object } => {
            doc.remove_object(object.id)?;
            Some(Command::CreateObject {
                object: object.clone(),
            })
        }
        Command::MoveObjects { ids, dx, dy } => {
            doc.move_objects(ids, *dx, *dy);
            Some(Command::MoveObjects {
                ids: ids.clone(),
                dx: -*dx,
                dy: -*dy,
            })
        }
        Command::UpdateText { id, before, after } => {
            let object = doc.objects.get_mut(id)?;
            object.text = after.clone();
            Some(Command::UpdateText {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            })
        }
    }
}
