const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const BASE_URL = 'http://localhost:8080';
const SCREENSHOT_DIR = 'C:\\dev\\parkhub\\screenshots';

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function run() {
  // Ensure screenshot dir exists
  if (!fs.existsSync(SCREENSHOT_DIR)) {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
  }

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    locale: 'de-DE'
  });
  const page = await context.newPage();

  console.log('Testing ParkHub UI...\n');

  // 1. Login Page
  console.log('1. Login Page');
  await page.goto(BASE_URL + '/login');
  await sleep(1000);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '01-login.png'), fullPage: true });
  console.log('   Screenshot saved');

  // 2. Register Page
  console.log('2. Register Page');
  await page.goto(BASE_URL + '/register');
  await sleep(1000);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '02-register.png'), fullPage: true });
  console.log('   Screenshot saved');

  // Try to register a test user
  console.log('3. Registering test user...');
  try {
    await page.fill('input[placeholder*="Mustermann"]', 'Test User');
    await page.fill('input[placeholder*="maxmuster"]', 'testuser');
    await page.fill('input[placeholder*="beispiel"]', 'test@example.com');
    await page.fill('input[placeholder="••••••••"]', 'password123');
    // Find confirm password field
    const passwordFields = await page.locator('input[type="password"]').all();
    if (passwordFields.length > 1) {
      await passwordFields[1].fill('password123');
    }
    await page.click('button[type="submit"]');
    await sleep(2000);
  } catch (e) {
    console.log('   Registration might have issues:', e.message);
  }
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '03-after-register.png'), fullPage: true });

  // Check if we're logged in (redirected to dashboard)
  const currentUrl = page.url();
  console.log('   Current URL:', currentUrl);

  if (currentUrl.includes('/login') || currentUrl.includes('/register')) {
    // Try login instead
    console.log('4. Trying login...');
    await page.goto(BASE_URL + '/login');
    await sleep(500);
    try {
      await page.fill('input[type="text"], input[placeholder*="Benutzername"]', 'testuser');
      await page.fill('input[type="password"]', 'password123');
      await page.click('button[type="submit"]');
      await sleep(2000);
    } catch (e) {
      console.log('   Login form issue:', e.message);
    }
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, '04-after-login.png'), fullPage: true });
  }

  // Dashboard
  console.log('5. Dashboard');
  await page.goto(BASE_URL + '/');
  await sleep(1500);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '05-dashboard.png'), fullPage: true });

  // Book Page
  console.log('6. Book Page');
  await page.goto(BASE_URL + '/book');
  await sleep(1500);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '06-book.png'), fullPage: true });

  // Bookings Page
  console.log('7. Bookings Page');
  await page.goto(BASE_URL + '/bookings');
  await sleep(1500);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '07-bookings.png'), fullPage: true });

  // Vehicles Page
  console.log('8. Vehicles Page');
  await page.goto(BASE_URL + '/vehicles');
  await sleep(1500);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '08-vehicles.png'), fullPage: true });

  // Admin Page
  console.log('9. Admin Page');
  await page.goto(BASE_URL + '/admin');
  await sleep(1500);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '09-admin.png'), fullPage: true });

  // Test Dark Mode
  console.log('10. Dark Mode Toggle');
  await page.goto(BASE_URL + '/');
  await sleep(1000);
  // Find and click dark mode toggle
  try {
    const moonIcon = await page.locator('[data-testid="theme-toggle"], button:has(svg)').first();
    if (moonIcon) {
      await moonIcon.click();
      await sleep(500);
    }
  } catch (e) {
    console.log('   Dark mode toggle not found');
  }
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, '10-dark-mode.png'), fullPage: true });

  await browser.close();
  
  console.log('\n✅ All screenshots saved to:', SCREENSHOT_DIR);
  console.log('\nFiles:');
  const files = fs.readdirSync(SCREENSHOT_DIR).filter(f => f.endsWith('.png'));
  files.forEach(f => console.log('  -', f));
}

run().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});
