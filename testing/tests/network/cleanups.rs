// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::wait_until;

use pea2pea::Pea2Pea;
use peak_alloc::PeakAlloc;
use snarkos_testing::{SnarkosNode, TestNode};

// Configure a custom allocator that will measure memory use.
#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

// A helper function making memory use more human-readable.
fn display_bytes(bytes: f64) -> String {
    const GB: f64 = 1_000_000_000.0;
    const MB: f64 = 1_000_000.0;
    const KB: f64 = 1_000.0;

    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes / KB)
    } else {
        format!("{:.2} B", bytes)
    }
}

#[tokio::test]
#[ignore = "this test is purely informational; latest result: 159.81 MB"]
async fn measure_node_overhead() {
    // Register initial memory use.
    let initial_mem = PEAK_ALLOC.current_usage();

    // Start a snarkOS node.
    let _snarkos_node = SnarkosNode::default().await;

    // Register memory use caused by the node.
    let node_mem_use = PEAK_ALLOC.current_usage() - initial_mem;

    // Display the result.
    println!("snarkOS node memory use: {}", display_bytes(node_mem_use as f64));
}

#[tokio::test]
#[ignore = "TODO: indicates a potential leak (13420B - around 1.3kB/connection); investigate further"]
async fn inbound_connect_and_disconnect_doesnt_leak() {
    // Start a test node.
    let test_node = TestNode::default().await;

    // Start a snarkOS node.
    let snarkos_node = SnarkosNode::default().await;

    // Register initial memory use.
    let pre_connection_mem = PEAK_ALLOC.current_usage();

    // Perform a connect and disconnect several times.
    let mut first_conn_mem = None;
    for i in 0..10 {
        // Connect the test node to the snarkOS node (inbound for snarkOS).
        test_node.node().connect(snarkos_node.local_addr()).await.unwrap();

        // Disconnect the test node from the snarkOS node.
        assert!(test_node.node().disconnect(snarkos_node.local_addr()).await);
        wait_until!(1, snarkos_node.connected_peers().await.is_empty());
        snarkos_node.reset_known_peers().await;

        if i == 0 {
            // Measure memory use caused by the 1st connect and disconnect.
            first_conn_mem = Some(PEAK_ALLOC.current_usage());
            println!(
                "Memory increase after a single outbound connection: {}",
                display_bytes((first_conn_mem.unwrap() - pre_connection_mem) as f64)
            );
        }
    }

    // Measure memory use after the repeated connections.
    let final_mem = PEAK_ALLOC.current_usage();

    // Check if there is a connection-related leak.
    let leaked_mem = final_mem.saturating_sub(first_conn_mem.unwrap());
    assert_eq!(leaked_mem, 0);
}

#[tokio::test]
#[ignore = "TODO: indicates a potential leak (12424B - around 1.2kB/connection); investigate further"]
async fn outbound_connect_and_disconnect_doesnt_leak() {
    // Start a snarkOS node.
    let snarkos_node = SnarkosNode::default().await;

    // Start a test node.
    let test_node = TestNode::default().await;
    let test_node_addr = test_node.node().listening_addr().unwrap();

    // Register memory use before any connections.
    let pre_connection_mem = PEAK_ALLOC.current_usage();

    // Perform a connect and disconnect several times.
    let mut first_conn_mem = None;
    for i in 0..10 {
        // Connect the snarkOS node to the test node (outbound for snarkOS).
        snarkos_node.connect(test_node_addr).await.unwrap();

        // Disconnect the test node from the snarkOS node.
        wait_until!(1, test_node.node().num_connected() == 1);
        let snarkos_node_addr = test_node.node().connected_addrs()[0];
        assert!(test_node.node().disconnect(snarkos_node_addr).await);
        wait_until!(1, snarkos_node.connected_peers().await.is_empty());
        snarkos_node.reset_known_peers().await;

        if i == 0 {
            // Measure memory use caused by the 1st connect and disconnect.
            first_conn_mem = Some(PEAK_ALLOC.current_usage());
            println!(
                "Memory increase after a single outbound connection: {}",
                display_bytes((first_conn_mem.unwrap() - pre_connection_mem) as f64)
            );
        }
    }

    // Measure memory use after the repeated connections.
    let final_mem = PEAK_ALLOC.current_usage();

    // Check if there is a connection-related leak.
    let leaked_mem = final_mem.saturating_sub(first_conn_mem.unwrap());
    assert_eq!(leaked_mem, 0);
}
