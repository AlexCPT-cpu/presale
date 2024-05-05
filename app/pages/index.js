import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import useMount from "@/hooks/useMount";
import { useState } from "react";
import FlipClockCountdown from "@leenguyen/react-flip-clock-countdown";
import "@leenguyen/react-flip-clock-countdown/dist/index.css";

export default function Home() {
  const mounted = useMount();
  const [amount, setAmount] = useState(0); // State for input amount
  const [totalTokensBought, setTotalTokensBought] = useState(0); // State for total tokens bought
  const [tokensReleased, setTokensReleased] = useState(0); // State for released tokens

  const handleBuyTokens = () => {
    // Add your logic here to handle buying tokens
    // For now, just update the total tokens bought and released tokens with the input amount
    setTotalTokensBought(totalTokensBought + parseInt(amount));
    setTokensReleased(tokensReleased + parseInt(amount));
  };

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
            <p className="text-lg">Total Tokens Bought: {totalTokensBought}</p>
          </div>
          <div className="border border-green-500 rounded-lg p-4">
            <p className="text-lg">Tokens Released (TGE): {tokensReleased}</p>
          </div>
        </div>

        {/* Input field for user to input amount */}
        <div className="flex flex-col items-center mb-4 mt-2 lg:mt-10">
          <label htmlFor="amount" className="text-lg mb-2">
            Enter Amount to Buy:
          </label>
          <input
            type="number"
            id="amount"
            className="bg-gray-700 text-white px-4 py-2 rounded-md w-64 outline-none appearance-none hover:appearance-none"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
          />
        </div>

        {/* Button to buy tokens */}
        <button
          className="bg-[#512da8] text-white px-9 py-3 rounded-md shadow-md hover:bg-gray-900 transition duration-300"
          onClick={handleBuyTokens}
        >
          Buy Tokens
        </button>
      </div>
    </div>
  );
}
