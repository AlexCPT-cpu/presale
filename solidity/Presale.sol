//pragma solidity ^0.8.0;

//import "./IERC20.sol";

contract PresaleAndVesting {

    address authority;
    address public beneficiary;
    uint64 public totalTokens;
    uint64 public soldTokens;
    uint64 public tokenPrice;

struct VestingSchedule {
    uint64 startTimestamp;
    uint64 cliffDuration;
    uint64 totalAmount;
    uint64 lastClaimTimestamp;
}

    mapping(address => uint64) public balances;
    mapping(address => VestingSchedule) public vestingSchedules;

    bool public presaleClosed;

    event TokensPurchased(address indexed buyer, uint64 amount);
    event TokensClaimed(address indexed beneficiary, uint64 amount);
    event PresaleClosed();
    event EtherWithdrawn(uint64 amount);
    event TokensDeposited(uint64 amount);

    @payer(payer)
    constructor(address _beneficiary, uint64 _totalTokens, uint64 _tokenPrice,address _initial_authority) {
        beneficiary = _beneficiary;
        totalTokens = _totalTokens;
        tokenPrice = _tokenPrice;
        authority = _initial_authority;
    }

    @signer(authorityAccount)
    function set_new_authority(address _new_authority) external {
        assert(tx.accounts.authorityAccount.key == authority && tx.accounts.authorityAccount.is_signer);
        authority = _new_authority;
    }

    @signer(authorityAccount)

    function purchaseTokens() external payable {
        require(!presaleClosed, "Presale is closed");
        require(msg.value > 0, "Invalid amount");
        require(soldTokens < totalTokens, "Presale sold out");

        uint64 remainingTokens = totalTokens - soldTokens;
        uint64 purchaseAmount = msg.value * tokenPrice;
        uint64 tokensToPurchase = purchaseAmount > remainingTokens ? remainingTokens : purchaseAmount;

        balances[msg.sender] += tokensToPurchase;
        soldTokens += tokensToPurchase;

        uint64 refundAmount = msg.value - (tokensToPurchase / tokenPrice);
        if (refundAmount > 0) {
            payable(msg.sender).transfer(refundAmount);
        }

        if (soldTokens == totalTokens) {
            presaleClosed = true;
            emit PresaleClosed();
        }

        emit TokensPurchased(msg.sender, tokensToPurchase);
    }

    function claimVestedTokens() external {
        require(!presaleClosed, "Presale is closed");
        uint64 vestedAmount = calculateVestedAmount(msg.sender);
        require(vestedAmount > 0, "No vested tokens available");

        balances[msg.sender] -= vestedAmount;
        IERC20(beneficiary).transfer(msg.sender, vestedAmount);

        emit TokensClaimed(msg.sender, vestedAmount);
    }

    function calculateVestedAmount(address account) internal view returns (uint64) {
        VestingSchedule storage vestingSchedule = vestingSchedules[account];
        require(block.timestamp >= vestingSchedule.startTimestamp, "Vesting not started yet");

        uint64 elapsedTime = block.timestamp - vestingSchedule.startTimestamp;
        if (elapsedTime < vestingSchedule.cliffDuration) {
            return 0; // No tokens vested during cliff period
        }

        uint64 elapsedMonths = elapsedTime / 30 days;
        uint64 vestedAmount = 0;

        for (uint64 i = 0; i < elapsedMonths; i++) {
            uint64 unlockPercentage = (i + 1) * 10; // 10% for the first month, 20% for the second, and so on
            uint64 unlockAmount = (vestingSchedule.totalAmount * unlockPercentage) / 100;
            uint64 alreadyClaimed = (vestingSchedule.totalAmount * (i * 10)) / 100;

            if (alreadyClaimed < unlockAmount && vestingSchedule.lastClaimTimestamp + (30 days * (i + 1)) <= block.timestamp) {
                vestedAmount += unlockAmount - alreadyClaimed;
                vestingSchedule.lastClaimTimestamp = block.timestamp;
            }
        }

        return vestedAmount;
    }

    function getTotalTokensSold() external view returns (uint64) {
        return soldTokens;
    }

    function getTotalTokensRemaining() external view returns (uint64) {
        return totalTokens - soldTokens;
    }

    function isPresaleClosed() external view returns (bool) {
        return presaleClosed;
    }

    function withdrawEther(uint64 amount) external {
        require(msg.sender == beneficiary, "Only beneficiary can withdraw");
        require(amount <= address(this).balance, "Insufficient balance");

        payable(beneficiary).transfer(amount);
        emit EtherWithdrawn(amount);
    }

    function depositTokens(uint64 amount) external {
        require(msg.sender == beneficiary, "Only beneficiary can deposit");
        require(!presaleClosed, "Presale is closed");

        IERC20(beneficiary).transferFrom(msg.sender, address(this), amount);
        emit TokensDeposited(amount);
    }
}
