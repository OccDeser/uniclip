
pub struct Clipboard {
    pub content: Vec<Vec<u8>>,
    pub capacity: u32,
    pub size: u32,
    pub updated: bool,
}

impl Clipboard {
    pub fn new(capacity: u32) -> Self {
        Self {
            content: Vec::new(),
            capacity,
            size: 0,
            updated: false,
        }
    }

    // pub fn first(&self) -> Option<&Vec<u8>> {
    //     self.content.first()
    // }

    // pub fn last(&self) -> Option<&Vec<u8>> {
    //     self.content.last()
    // }

    // pub fn get(&self, index: usize) -> Option<&Vec<u8>> {
    //     self.content.get(index)
    // }

    // pub fn remove(&mut self, index: usize) {
    //     if index < self.content.len() {
    //         self.content.remove(index);
    //         self.size -= 1;
    //     }
    // }

    pub fn append(&mut self, data: Vec<u8>) {
        while self.size >= self.capacity {
            self.size -= 1;
            self.content.pop();
        }
        self.content.insert(0, data);
        self.size += 1;
        self.updated = true;
    }

    // pub fn clear(&mut self) {
    //     self.content.clear();
    //     self.size = 0;
    // }
}