/**
 * mock-data.js - Hard-coded mock datasets
 */

const _mockData = (() => {
  const resellers = ['HealthFirst', 'Elite Medical', 'Wellness Partners', 'ProCare', 'DirectMed'];
  const lawFirms = ['Barker & Associates', 'The Smith Law Group', 'Justice Legal', 'Prestige Law', 'Miller & Sons'];
  const carriers = ['BlueShield', 'Aetna', 'UnitedHealth', 'Cigna', 'StateFarm'];
  const chiropractors = ['Dr. John Doe', 'Dr. Jane Smith', 'Dr. Bob Brown', 'Dr. Alice Green'];
  const doctors = ['Dr. Sarah White', 'Dr. Mike Black', 'Dr. Emily Rose', 'Dr. Tom Gray'];
  const statuses = ['Scheduled', 'Completed', 'No Show', 'Missing Info', 'Missing Paperwork', 'Needs Review', 'Approved', 'Declined'];

  const customers = [];
  for (let i = 1; i <= 150; i++) {
    customers.push({
      id: `CUST-${1000 + i}`,
      fullName: `Customer ${i}`,
      phone: `555-01${(i % 100).toString().padStart(2, '0')}`,
      email: `user${i}@example.com`,
      city: ['Miami', 'Orlando', 'Tampa', 'Jacksonville'][i % 4],
      state: 'FL',
      reseller: resellers[i % resellers.length],
      lawFirm: lawFirms[i % lawFirms.length],
      carrier: carriers[i % carriers.length],
      flags: {
        noShow: i % 10 === 0,
        missingInfo: i % 15 === 0,
        missingPaperwork: i % 12 === 0
      },
      createdAt: new Date(Date.now() - (i * 86400000)).toISOString(),
      lastUpdated: new Date().toISOString()
    });
  }

  const appointments = [];
  const now = Date.now();
  for (let i = 1; i <= 250; i++) {
    const cust = customers[i % customers.length];
    const date = new Date(now - (Math.floor(Math.random() * 90) * 86400000));
    appointments.push({
      id: `APT-${2000 + i}`,
      customerId: cust.id,
      apptDate: date.toISOString(),
      apptType: i % 3 === 0 ? 'Home' : 'Office',
      status: statuses[i % statuses.length],
      chiroName: chiropractors[i % chiropractors.length],
      doctorName: doctors[i % doctors.length],
      coverageAmount: Math.floor(Math.random() * 5000) + 500,
      referralOwed: Math.floor(Math.random() * 500),
      notes: `Notes for appointment ${i}`
    });
  }

  return {
    customers,
    appointments,
    resellers,
    lawFirms,
    carriers,
    chiropractors,
    doctors,
    statuses
  };
})();

// Export to window for access
window.MOCK_DATA = _mockData;
