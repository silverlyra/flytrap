#[derive(Debug)]
pub enum ServerMessage {
    Hello(Hello),
    Update(Update),
}

#[derive(Debug)]
pub struct Hello;

#[derive(Debug)]
pub struct Update;
