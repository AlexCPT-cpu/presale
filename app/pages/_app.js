import { PhantomWalletAdapter } from "@solana/wallet-adapter-phantom";
import {
  ConnectionProvider,
  WalletProvider,
} from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import "@solana/wallet-adapter-react-ui/styles.css";
import "@/styles/globals.css";
import { clusterApiUrl } from "@solana/web3.js";
import { endpoint } from "@/constants";
import { Toaster } from "react-hot-toast";

const rpcEndpoint = clusterApiUrl("devnet"); // Specify the Solana network

export default function App({ Component, pageProps }) {
  const phantomWallet = new PhantomWalletAdapter();
  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={[phantomWallet]} autoConnect>
        <WalletModalProvider>
          <Component {...pageProps} />
          <Toaster />
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
