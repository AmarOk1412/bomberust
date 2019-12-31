/**
 * Copyright (c) 2019, Sébastien Blin <sebastien.blin@enconn.fr>
 * All rights reserved.
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * * Redistributions of source code must retain the above copyright
 *  notice, this list of conditions and the following disclaimer.
 * * Redistributions in binary form must reproduce the above copyright
 *  notice, this list of conditions and the following disclaimer in the
 *  documentation and/or other materials provided with the distribution.
 * * Neither the name of the University of California, Berkeley nor the
 *  names of its contributors may be used to endorse or promote products
 *  derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND ANY
 * EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE REGENTS AND CONTRIBUTORS BE LIABLE FOR ANY
 * DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
 * (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
 * ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 **/
use rand::Rng;

/**
 * Represents a neuron. Take inputs, apply weights, return the sigmoid result (between 0 and 1)
 */
#[derive(Clone, Deserialize, Serialize)]
pub struct Neuron {
    pub weights: Vec<f32>,
}

impl Neuron {
    /**
     * Creates a new neuron
     * @param inputs_len    Inputs length
     * @return the new neuron
     */
    pub fn new(inputs_len: usize) -> Neuron {
        let mut weights = Vec::with_capacity(inputs_len);
        let mut rng = rand::thread_rng();
        for _ in 0..inputs_len {
            weights.push(rng.gen_range(-1.0, 1.0));
        }
        Neuron {
            weights,
        }
    }

    /**
     * Util to multiply two vectors
     * @param v1    First vector
     * @param v2    Second vector
     * @return  A vector equals to v1.v2
     */
    fn vec_mul<T>(v1: &[T], v2: &[T]) -> Vec<T>
    where
        T: std::ops::Mul<Output = T> + Copy,
    {
        if v1.len() != v2.len() {
            panic!("Cannot multiply vectors of different lengths!")
        }

        v1.iter().zip(v2).map(|(&i1, &i2)| i1 * i2).collect()
    }

    /**
     * Returns the sigmoid result
     * @param x     Input
     * @return sigmoid result
     */
    fn sigmoid(self, x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }

    /**
     * Calculate the output from the inputs
     * @param inputs    A Vector containing the inputs
     * @return the f32 output
     */
    pub fn calc(self, inputs: &Vec<f32>) -> f32 {
        let weights = self.weights.clone();
        self.sigmoid(Neuron::vec_mul(inputs, &weights).iter().sum())
    }
}

/**
 * Represent a single depth neural network
 */
#[derive(Clone, Deserialize, Serialize)]
pub struct NeuralNetwork {
    pub structure: Vec<usize>,
    pub neurons: Vec<Neuron>,
    pub neurons_len: usize,
    pub largest_layer: usize,
}

impl NeuralNetwork {
    /**
     * Creates a new neural network following the giver structure.
     * For example structure = vec![5,32,2] will create a 3 layers
     * neural network (with respectively 5 inputs, 32 neurons, 2 outputs).
     * @param structure     Structure of the network
     * @return a new NeuralNetwork
     */
    pub fn new(structure: Vec<usize>) -> NeuralNetwork {
        let mut neurons = Vec::new();
        let neurons_len: usize = structure.iter().sum();
        let largest_layer: usize = *structure.iter().max().unwrap();
        let mut idx = 0;
        for layer in &structure {
            // Add neurons
            for _ in 0..*layer {
                let mut inputs_len: usize = 0;
                if idx != 0 {
                    inputs_len = structure[idx-1];
                }
                neurons.push(Neuron::new(inputs_len));
            }
            idx += 1;
        }
        NeuralNetwork {
            structure,
            neurons,
            neurons_len,
            largest_layer
        }
    }

    /**
     * Calculates the output of a network
     * @param inputs    Inputs data
     * @return the result with a vector of f32 with the size
     * of the output layer
     */
    pub fn calc(self, inputs: Vec<f32>) -> Vec<f32> {
        let mut result = Vec::with_capacity(*self.structure.last().unwrap());
        let mut temporary_result = Vec::<f32>::with_capacity(self.neurons_len);

        let mut idx = 0;
        let mut neuron_idx : usize = 0;
        let mut input = Vec::<f32>::with_capacity(self.largest_layer);
        for layer in &self.structure {
            let start = neuron_idx;
            let is_last_layer = idx == self.structure.len() - 1;
            for n in 0..*layer {
                // Inputs are just stored
                if idx == 0 {
                    temporary_result.push(inputs[n]);
                } else {
                    let output = self.neurons[neuron_idx].clone().calc(&input);
                    if is_last_layer {
                        result.push(output);
                    } else {
                        temporary_result.push(output);
                    }
                }
                neuron_idx += 1;
            }
            idx += 1;
            if !is_last_layer {
                input = temporary_result[start..neuron_idx].to_vec();
            }
        }

        result
    }

    pub fn mutate(&mut self) {
        let mut rng = rand::thread_rng();
        for i in 0..self.neurons_len {
            if rng.gen_range(0, 100) <= 5 {
                let inputs_len = self.neurons[i].weights.len();
                self.neurons[i] = Neuron::new(inputs_len);
            }
        }
    }

    pub fn cross(&mut self, other: &NeuralNetwork) -> NeuralNetwork {
        let result = self.clone();
        let mut rng = rand::thread_rng();
        for i in 0..self.neurons_len {
            if rng.gen_range(0, 100) < 50 {
                self.neurons[i] = other.neurons[i].clone();
            }
        }
        result
    }
}
