use shared_bot_infrastructure::{DynamicFeeModel, FeeCalculation};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let fee_model = DynamicFeeModel::new();

    println!("🔢 Dynamic Fee Model - Profit Tier Analysis\n");

    // Test different profit scenarios
    let test_scenarios = vec![
        (0.3, 0.01, "Below minimum threshold"),
        (0.7, 0.02, "Small profit (0.5-1 SOL tier)"),
        (1.5, 0.03, "Medium profit (1-3 SOL tier)"),
        (2.8, 0.05, "Upper medium profit"),
        (4.2, 0.08, "Large profit (3+ SOL tier)"),
        (10.0, 0.15, "Very large profit"),
    ];

    for (profit_sol, dex_fees_sol, description) in test_scenarios {
        println!("📊 Scenario: {}", description);
        println!("   Gross Profit: {:.3} SOL", profit_sol);
        println!("   DEX Fees: {:.3} SOL", dex_fees_sol);

        let calculation = fee_model.calculate_fees(profit_sol, dex_fees_sol)?;

        print_fee_calculation(&calculation);

        // Test Jito tip calculation with varying conditions
        let low_congestion_tip = fee_model.calculate_jito_tip(profit_sol, 0.2, 3);
        let high_congestion_tip = fee_model.calculate_jito_tip(profit_sol, 0.9, 8);

        println!("   🚦 Jito Tips:");
        println!("      Low congestion: {:.4} SOL", low_congestion_tip);
        println!("      High congestion: {:.4} SOL", high_congestion_tip);

        let priority = fee_model.get_priority_boost(profit_sol);
        println!("   🎯 Priority Boost: {}/10", priority);

        println!("   {}", "─".repeat(50));
        println!();
    }

    // Show profit tier boundaries
    println!("📈 Profit Tier Summary:");
    println!("   • 0.5-1.0 SOL: 1.2x multiplier, 8% gas, max 0.05 SOL tip");
    println!("   • 1.0-3.0 SOL: 1.15x multiplier, 10% gas, max 0.15 SOL tip");
    println!("   • 3.0+ SOL: 1.1x multiplier, 12% gas, max 0.5 SOL tip");

    Ok(())
}

fn print_fee_calculation(calc: &FeeCalculation) {
    let status = if calc.should_execute {
        "✅ EXECUTE"
    } else {
        "❌ SKIP"
    };
    let multiplier = calc.net_profit_sol / calc.total_fees_sol;

    println!("   Result: {} ({})", status, calc.tier_name);
    println!("   Net Profit: {:.4} SOL", calc.net_profit_sol);
    println!(
        "   Total Fees: {:.4} SOL (Gas: {:.4}, DEX: {:.4})",
        calc.total_fees_sol, calc.gas_tip_sol, calc.dex_fees_sol
    );
    println!(
        "   Actual Multiplier: {:.2}x (Required: {:.2}x)",
        multiplier, calc.profit_multiplier
    );
}
