import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { WalletNotConnectedError } from "@solana/wallet-adapter-base";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  Connection,
  sendAndConfirmTransaction,
  VersionedTransaction,
  TransactionMessage,
} from "@solana/web3.js";
import React, { FC, useCallback, useEffect } from "react";
import useMount from "@/hooks/useMount";
import { useState } from "react";
import FlipClockCountdown from "@leenguyen/react-flip-clock-countdown";
import "@leenguyen/react-flip-clock-countdown/dist/index.css";
import toast from "react-hot-toast";

export default function Home() {
  const mounted = useMount();
  const { connection } = useConnection();
  const { publicKey, sendTransaction, wallet } = useWallet();
  const [amount, setAmount] = useState(0); // State for input amount
  const [tokenAmount, setTokenAmount] = useState(0);
  const [totalTokensBought, setTotalTokensBought] = useState(0); // State for total tokens bought
  const [tokensReleased, setTokensReleased] = useState(0); // State for released tokens
  const [recipientAddress, setRecipientAddress] = useState(
    "HmwhDijzFs1uBw7h8yWqcZvhLNUuJR61sSidRDabp7kz"
  );

  // Define program ID and payer account

  useEffect(() => {
    const tokens = localStorage.getItem("tokens_bought");
    setTotalTokensBought(tokens);
    setTokensReleased(0.2 * Number(tokens));
  }, []);

  const handleBuyTokens = () => {
    setTotalTokensBought(totalTokensBought + parseInt(amount));
    setTokensReleased(tokensReleased + parseInt(amount));
  };

  // Function to create a new campaign account
  const createCampaignAccount = useCallback(async () => {
    try {
      const programId = new PublicKey(
        "Ew9dAwC9poiMPCEVf3RJeSTnSNCo3j4qFp8yR4x35S4U"
      );
      // Generate a new keypair for the campaign account
      const campaignAccount = Keypair.generate();

      const accounts = [
        {
          pubkey: publicKey,
          isSigner: true,
          isWritable: true,
        },
        {
          pubkey: campaignAccount.publicKey,
          isSigner: false,
          isWritable: true,
        },
        // Add other accounts if needed (e.g., user account)
      ];

      // Define initial lamports and space
      const lamports = 1000000; // Example: 1 SOL (in lamports)
      const space = 8192; // Example: 8 KB

      let latestBlockhash = await connection.getLatestBlockhash();

      // Construct the transaction
      const transaction = new Transaction({
        recentBlockhash: latestBlockhash,
      })
        .add(
          SystemProgram.createAccount({
            fromPubkey: publicKey,
            newAccountPubkey: campaignAccount.publicKey,
            lamports: lamports,
            space: space,
            programId: programId,
          })
        )
        .add({
          accounts: accounts.map((acc) => ({
            pubkey: acc.pubkey,
            isSigner: acc.isSigner,
            isWritable: acc.isWritable,
          })),
        });

      // Sign and send the transaction
      const signature = await window.solana.signTransaction(transaction);
      const txid = await connection.sendRawTransaction(signature.serialize());
      console.log("Transaction ID:", txid);
    } catch (error) {
      console.error("Error creating campaign account:", error);
    }
  }, [connection, publicKey]);
  const createCampaignAccount2 = useCallback(async () => {
    try {
      const programId = new PublicKey(
        "Ew9dAwC9poiMPCEVf3RJeSTnSNCo3j4qFp8yR4x35S4U"
      );
      // Generate a new keypair for the campaign account
      const campaignAccount = Keypair.generate();

      const accounts = [
        {
          pubkey: publicKey,
          isSigner: true,
          isWritable: true,
        },
        {
          pubkey: campaignAccount.publicKey,
          isSigner: false,
          isWritable: true,
        },
        // Add other accounts if needed (e.g., user account)
      ];

      // Define initial lamports and space
      const lamports = 1000000; // Example: 1 SOL (in lamports)
      const space = 8192; // Example: 8 KB

      let latestBlockhash = await connection.getLatestBlockhash();

      const createAccountTransaction = new Transaction({
        recentBlockhash: latestBlockhash,
        feePayer: publicKey,
      })
        .add(
          SystemProgram.createAccount({
            fromPubkey: publicKey,
            newAccountPubkey: campaignAccount.publicKey,
            lamports: lamports,
            space: space,
            programId: programId,
          })
        )
        .add({
          accounts: accounts.map((acc) => ({
            pubkey: acc.pubkey,
            isSigner: acc.isSigner,
            isWritable: acc.isWritable,
          })),
        });
      const signature = await window.solana.signTransaction(
        createAccountTransaction
      );
      const txid = await connection.sendRawTransaction(signature.serialize());
      console.log("Transaction ID:", txid);
      console.log(publicKey, campaignAccount.publicKey);
    } catch (error) {
      console.error("Error creating campaign account:", error);
    }
  }, [connection, publicKey]);

  const updateTokens = () => {
    const tokens = localStorage.getItem("tokens_bought");
    setTotalTokensBought(tokens);
    setTokensReleased(0.2 * Number(tokens));
  };
  // Function to send SOL
  const sendSol = useCallback(async () => {
    if (!publicKey) {
      toast.error(`Wallet not connected!`);
      console.log("error", `Send Transaction: Wallet not connected!`);
      return;
    }

    let signature = "";
    const recipient = new PublicKey(recipientAddress);
    try {
      // Create instructions to send, in this case a simple transfer
      const instructions = [
        SystemProgram.transfer({
          fromPubkey: publicKey,
          toPubkey: recipient,
          lamports: amount,
        }),
      ];

      // Get the lates block hash to use on our transaction and confirmation
      let latestBlockhash = await connection.getLatestBlockhash();

      // Create a new TransactionMessage with version and compile it to legacy
      const messageLegacy = new TransactionMessage({
        payerKey: publicKey,
        recentBlockhash: latestBlockhash.blockhash,
        instructions,
      }).compileToLegacyMessage();

      // Create a new VersionedTransacction which supports legacy and v0
      const transation = new VersionedTransaction(messageLegacy);

      // Send transaction and await for signature
      signature = await sendTransaction(transation, connection);

      // Send transaction and await for signature
      await connection.confirmTransaction(
        { signature, ...latestBlockhash },
        "confirmed"
      );

      console.log(signature);
      const tAmount = localStorage.getItem("tokens_bought", String(amount));
      const total_amount = Number(tAmount) + tokenAmount;
      localStorage.setItem("tokens_bought", total_amount);
      // notify({
      //   type: "success",
      //   message: "Transaction successful!",
      //   txid: signature,
      // });
      toast.success(`Transaction successful!`);
      setAmount(0);
      setTokenAmount(0);
      updateTokens();
    } catch (error) {
      // notify({
      //   type: "error",
      //   message: `Transaction failed!`,
      //   description: error?.message,
      //   txid: signature,
      // });
      toast.error(`Transaction failed! Or Execution Cancelled`);
      console.log("error", `Transaction failed! ${error?.message}`, signature);
      return;
    }
  }, [
    publicKey,
    connection,
    sendTransaction,
    amount,
    recipientAddress,
    tokenAmount,
  ]);

  return (
    <div className="p-10 bg-black min-h-screen text-white">
      <div className="flex justify-end w-full">
        {mounted && <WalletMultiButton />}
      </div>

      <div className="mt-20 lg:mt-32 bg-black text-white flex flex-col justify-center items-center w-full">
        <h1 className="text-4xl mb-4"></h1>
        <p className="mb-10 font-semibold text-2xl lg:text-4xl capitalize">
          The token sale is ongoing.
        </p>
        <div className="hidden w-full lg:flex h-full justify-center items-center">
          <FlipClockCountdown
            labelStyle={{
              fontSize: 12,
              fontWeight: 700,
              textTransform: "uppercase",
            }}
            digitBlockStyle={{ width: 55, height: 70, fontSize: 30 }}
            dividerStyle={{ color: "white", height: 1 }}
            separatorStyle={{ color: "#512da8", size: "6px" }}
            to={new Date().getTime() + 24 * 3600 * 1000 + 5000}
          />
        </div>
        <div className="w-full flex lg:hidden h-full justify-center items-center">
          <FlipClockCountdown
            labelStyle={{
              fontSize: 11,
              fontWeight: 600,
              textTransform: "uppercase",
            }}
            digitBlockStyle={{ width: 32, height: 65, fontSize: 30 }}
            dividerStyle={{ color: "white", height: 1 }}
            separatorStyle={{ color: "#512da8", size: "6px" }}
            to={new Date().getTime() + 24 * 3600 * 1000 + 5000}
          />
        </div>

        {/* Counter for token sale */}
        <div className="mt-10 mb-16">
          <div className="border border-[#512da8] rounded-lg p-4 mb-4">
            <p className="text-lg">
              Total Tokens Bought: {totalTokensBought ? totalTokensBought : 0}
            </p>
          </div>
          <div className="border border-green-500 rounded-lg p-4">
            <p className="text-lg">
              Tokens Released (TGE): {tokensReleased ? tokensReleased : 0}
            </p>
          </div>
        </div>

        <div>Price per token: 0.1 SOl</div>

        {/* Input field for user to input amount */}
        <div className="flex flex-col items-center mb-4 mt-2 lg:mt-10">
          <label htmlFor="amount" className="text-lg mb-2">
            Enter Amount to Buy:
          </label>
          <input
            type="number"
            id="amount"
            className="bg-gray-700 text-white px-4 py-2 rounded-md w-64 outline-none appearance-none hover:appearance-none"
            //value={amount}
            onChange={(e) => {
              setAmount(Number(e.target.value) * Number(0.01) * 1e9);
              setTokenAmount(Number(e.target.value));
            }}
          />
        </div>

        {/* Button to buy tokens */}
        <button
          className="bg-[#512da8] text-white px-9 py-3 rounded-md shadow-md hover:bg-gray-900 transition duration-300"
          onClick={sendSol}
        >
          Buy Tokens
        </button>

        <button
          className="bg-[#512da8] text-white px-9 py-3 rounded-md shadow-md hover:bg-gray-900 transition duration-300 mt-10"
          onClick={createCampaignAccount2}
        >
          Create Campaign
        </button>
      </div>
    </div>
  );
}
