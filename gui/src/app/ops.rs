use terrafier_core::ops::operations::Operation;

use crate::app::TerrafierApp;

impl TerrafierApp {
    pub fn save_for_undo(&mut self, op: Box<dyn Operation>) {
        self.undo_stack.push_back(op);
        if self.undo_stack.len() > 50 {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(op) = self.undo_stack.pop_back() {
            if let Some(ref mut world) = self.world {
                if let Some(dim) = world.overworld_mut() {
                    let inv = op.inverse();
                    if let Err(e) = inv.apply(dim) {
                        self.status_message = format!("Undo error: {:?}", e);
                        self.undo_stack.push_back(op);
                        return;
                    }
                    self.redo_stack.push(op);
                    self.status_message = "Undo".to_string();
                    return;
                }
            }
            self.undo_stack.push_back(op);
            self.status_message = "Undo error: world has no overworld".to_string();
        } else {
            self.status_message = "Nothing to undo".to_string();
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.redo_stack.pop() {
            if let Some(ref mut world) = self.world {
                if let Some(dim) = world.overworld_mut() {
                    if let Err(e) = op.apply(dim) {
                        self.status_message = format!("Redo error: {:?}", e);
                        self.redo_stack.push(op);
                        return;
                    }
                    self.undo_stack.push_back(op);
                    self.status_message = "Redo".to_string();
                    return;
                }
            }
            self.redo_stack.push(op);
            self.status_message = "Redo error: world has no overworld".to_string();
        } else {
            self.status_message = "Nothing to redo".to_string();
        }
    }
}
