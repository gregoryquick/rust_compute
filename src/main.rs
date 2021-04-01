mod pipelines;

use rand::prelude::*;

use std::thread;
//use std::time;
use futures::executor::block_on;

//const STACK_SIZE: usize = 8192 * 1024 * 1024;
const STACK_SIZE: usize = 4 * 1024 * 1024;

fn main() {
    // Spawn thread with explicit stack size
    let child = thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(run)
        .unwrap();

    // Wait for thread to join
    child.join().unwrap();}

fn run() {
    let mut rng = rand::thread_rng();
    let data = Vectors::new(&mut rng);    
    println!("A:");
    println!("{:?}", data.vec_a);
    println!("B:");
    println!("{:?}", data.vec_b);

    let mut pipeline_manager = block_on(pipelines::PipelineManager::new());
    //let now = time::Instant::now();
    println!("f(A, B):");
    let result = block_on(pipeline_manager.get_result(&data.vec_a, &data.vec_b)).unwrap();
    //println!("{}", now.elapsed().as_millis());
    println!("{:?}", result);
}


use pipelines::SIZE;

struct Vectors {
    vec_a: [f32; SIZE],
    vec_b: [f32; SIZE],
}

impl Vectors{
    fn new(rng: &mut rand::rngs::ThreadRng) -> Self {
        let mut vec_a: [f32; SIZE] = [0f32; SIZE];
        for num in vec_a.iter_mut() {
            *num = rng.gen();
        }
        let mut vec_b: [f32; SIZE] = [0f32; SIZE];
        for num in vec_b.iter_mut() {
            *num = rng.gen();
        }
        
        Vectors {
            vec_a,
            vec_b,
        }
    }
}
