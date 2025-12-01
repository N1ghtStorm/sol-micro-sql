import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolMicroSql } from "../target/types/sol_micro_sql";
import { expect } from "chai";
import { BN } from "@coral-xyz/anchor";

describe("sol-micro-sql", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.solMicroSql as Program<SolMicroSql>;
  const authority = anchor.Wallet.local().payer;

  // Helper function to get graph store PDA
  const getGraphStorePDA = async () => {
    const [graphStorePDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("graph_store")],
      program.programId
    );
    return graphStorePDA;
  };

  // Initialize graph before all tests
  before(async () => {
    const graphStorePDA = await getGraphStorePDA();

    let isInitialized = false;
    try {
      // Check if account already exists
      await program.account.graphStore.fetch(graphStorePDA);
      console.log("Graph store already initialized");
      isInitialized = true;
    } catch (err) {
      // Account doesn't exist, need to initialize
      console.log("Graph store does not exist, initializing...");
    }

    if (!isInitialized) {
      try {
        const tx = await program.methods
          .initializeGraph()
          .accountsPartial({
            graphStore: graphStorePDA,
            authority: authority.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .signers([authority])
          .rpc();

        console.log("Initialize transaction signature:", tx);

        // Wait a bit for the transaction to be confirmed
        await new Promise(resolve => setTimeout(resolve, 1000));

        // Fetch and verify the graph store account
        const graphStore = await program.account.graphStore.fetch(graphStorePDA);
        console.log("Graph store initialized successfully");
        console.log("Authority:", graphStore.authority.toString());
        console.log("Node count:", graphStore.nodeCount.toNumber());
        console.log("Edge count:", graphStore.edgeCount.toNumber());
      } catch (initErr: any) {
        console.error("Failed to initialize graph store:", initErr);
        // Try to get more details from the error
        if (initErr.logs) {
          console.error("Error logs:", initErr.logs);
        }
        if (initErr.error) {
          console.error("Error details:", JSON.stringify(initErr.error, null, 2));
        }
        // If initialization fails, try to continue anyway
        // The account might exist from a previous test run
        console.warn("Initialization failed, but continuing tests. Account might exist from previous run.");
        try {
          await program.account.graphStore.fetch(graphStorePDA);
          console.log("Account exists, tests can continue");
        } catch (fetchErr) {
          // If account really doesn't exist, we need to fail
          throw new Error(`Failed to initialize graph store and account does not exist: ${initErr.toString()}`);
        }
      }
    }
  });

  describe("initialize_graph", () => {
    it("Graph store is initialized", async () => {
      const graphStorePDA = await getGraphStorePDA();
      const graphStore = await program.account.graphStore.fetch(graphStorePDA);
      expect(graphStore.authority.toString()).to.equal(
        authority.publicKey.toString()
      );
    });
  });

  describe("execute_query", () => {
    it("Creates a node with CREATE query", async () => {
      const graphStorePDA = await getGraphStorePDA();

      const query = "CREATE (n:Person)";
      const result = await program.methods
        .executeQuery(query)
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      console.log("Create node transaction signature:", result);

      // Fetch and verify the graph store
      const graphStore = await program.account.graphStore.fetch(graphStorePDA);
      expect(graphStore.nodeCount.toNumber()).to.be.greaterThan(0);
      expect(graphStore.nodes.length).to.be.greaterThan(0);
    });

    it("Creates a node with hex data", async () => {
      const graphStorePDA = await getGraphStorePDA();

      const query = "CREATE (n:Person {0x1234})";
      const result = await program.methods
        .executeQuery(query)
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      console.log("Create node with data transaction signature:", result);

      // Fetch and verify the graph store
      const graphStore = await program.account.graphStore.fetch(graphStorePDA);
      const lastNode = graphStore.nodes[graphStore.nodes.length - 1];
      expect(lastNode.label).to.equal("Person");
      expect(Buffer.from(lastNode.data).toString("hex")).to.equal("1234");
    });

    it("Creates an edge between nodes", async () => {
      const graphStorePDA = await getGraphStorePDA();

      // Create first node
      await program.methods
        .executeQuery("CREATE (a:User)")
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      // Create second node
      await program.methods
        .executeQuery("CREATE (b:User)")
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      // Get node IDs from the graph (based on nonce)
      const graphStoreAfter = await program.account.graphStore.fetch(
        graphStorePDA
      );
      const node1Id = graphStoreAfter.nonce.subn(2).toString();
      const node2Id = graphStoreAfter.nonce.subn(1).toString();

      // Create edge between them
      const query = `CREATE (${node1Id})-[:FOLLOWS]->(${node2Id})`;
      const result = await program.methods
        .executeQuery(query)
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      console.log("Create edge transaction signature:", result);

      // Verify edge was created
      const graphStore = await program.account.graphStore.fetch(graphStorePDA);
      expect(graphStore.edgeCount.toNumber()).to.be.greaterThan(0);
      expect(graphStore.edges.length).to.be.greaterThan(0);
      const lastEdge = graphStore.edges[graphStore.edges.length - 1];
      expect(lastEdge.label).to.equal("FOLLOWS");
    });

    it("Executes MATCH query to find nodes", async () => {
      const graphStorePDA = await getGraphStorePDA();

      // First create a node
      await program.methods
        .executeQuery("CREATE (n:City)")
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      // Execute MATCH query
      const query = "MATCH (n:City) RETURN n.id LIMIT 10";
      const tx = await program.methods
        .executeQuery(query)
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      console.log("MATCH query transaction signature:", tx);

      // Verify the query executed successfully (no error thrown)
      // The result is returned but we can't easily access it in tests
      // We can verify the graph state is unchanged (MATCH is read-only)
      const graphStore = await program.account.graphStore.fetch(graphStorePDA);
      expect(graphStore.nodeCount.toNumber()).to.be.greaterThan(0);
    });

    it("Handles invalid query gracefully", async () => {
      const graphStorePDA = await getGraphStorePDA();

      const invalidQuery = "INVALID QUERY SYNTAX";

      try {
        await program.methods
          .executeQuery(invalidQuery)
          .accountsPartial({
            graphStore: graphStorePDA,
          })
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        // Check for QueryExecutionFailed error code or message
        const errStr = err.toString();
        const errorCode = err.error?.errorCode?.code;
        const hasError = errStr.includes("QueryExecutionFailed") || 
                        errStr.includes("queryExecutionFailed") ||
                        errStr.includes("Query execution failed") ||
                        errorCode === 6004;
        expect(hasError, `Expected QueryExecutionFailed error, got: ${errStr}`).to.be.true;
      }
    });
  });

  describe("get_node_info", () => {
    it("Gets information about an existing node", async () => {
      const graphStorePDA = await getGraphStorePDA();

      // Create a node first
      await program.methods
        .executeQuery("CREATE (n:TestNode)")
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      // Get the node ID from the graph
      const graphStore = await program.account.graphStore.fetch(graphStorePDA);
      const nodeId = graphStore.nonce.subn(1);

      // Get node info
      const result = await program.methods
        .getNodeInfo(nodeId)
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      console.log("Get node info transaction signature:", result);
    });

    it("Fails when node does not exist", async () => {
      const graphStorePDA = await getGraphStorePDA();

      const nonExistentNodeId = new BN("999999999999999999");

      try {
        await program.methods
          .getNodeInfo(nonExistentNodeId)
          .accountsPartial({
            graphStore: graphStorePDA,
          })
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        // Check for NodeNotFound error code or message
        const errStr = err.toString();
        const errorCode = err.error?.errorCode?.code;
        const hasError = errStr.includes("NodeNotFound") || 
                        errStr.includes("nodeNotFound") ||
                        errStr.includes("Node not found") ||
                        errorCode === 6001;
        expect(hasError, `Expected NodeNotFound error, got: ${errStr}`).to.be.true;
      }
    });
  });

  describe("Complex queries", () => {
    it("Creates multiple nodes and edges in sequence", async () => {
      const graphStorePDA = await getGraphStorePDA();

      // Create first node
      await program.methods
        .executeQuery("CREATE (a:User {0x0102})")
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      // Create second node
      await program.methods
        .executeQuery("CREATE (b:User {0x0304})")
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      // Get node IDs
      const graphStore = await program.account.graphStore.fetch(graphStorePDA);
      const node1Id = graphStore.nonce.subn(2).toString();
      const node2Id = graphStore.nonce.subn(1).toString();

      // Create edge
      await program.methods
        .executeQuery(`CREATE (${node1Id})-[:KNOWS]->(${node2Id})`)
        .accountsPartial({
          graphStore: graphStorePDA,
        })
        .rpc();

      // Verify final state
      const finalGraph = await program.account.graphStore.fetch(graphStorePDA);
      expect(finalGraph.nodeCount.toNumber()).to.equal(2);
      expect(finalGraph.edgeCount.toNumber()).to.equal(1);
      expect(finalGraph.nodes.length).to.equal(2);
      expect(finalGraph.edges.length).to.equal(1);
    });
  });
});
