const { getCurrentWindow } = window.__TAURI__.window;
const { invoke } = window.__TAURI__.core;

const appWindow = getCurrentWindow();

// --- CUSTOM TOAST FUNCTION --- //
function showToast(message, type = 'error') {
  const toast = document.getElementById('toast');
  const toastMsg = document.getElementById('toast-message');
  
  toastMsg.innerText = message;
  toast.className = `toast ${type} show`;
  
  setTimeout(() => {
    toast.className = `toast ${type}`;
  }, 3500);
}

document.addEventListener('DOMContentLoaded', () => {
  // Window Controls
  document.getElementById('minimize-btn').addEventListener('click', () => appWindow.minimize());
  document.getElementById('close-btn').addEventListener('click', () => appWindow.close());

  // Screen Views
  const homeView = document.getElementById('home-view');
  const addView = document.getElementById('add-view');
  const listView = document.getElementById('list-view');
  const accountsContainer = document.getElementById('accounts-container');

  // Buttons
  const addBtn = document.getElementById('add-btn');
  const listBtn = document.getElementById('list-btn');
  const removePatchesBtn = document.getElementById('remove-patches-btn'); // <-- Added this back!
  const backBtn = document.getElementById('back-btn');
  const listBackBtn = document.getElementById('list-back-btn');
  const submitAccountBtn = document.getElementById('submit-account-btn');

  // Input Fields
  const displayNameInput = document.getElementById('display-name');
  const skinNameInput = document.getElementById('skin-name');

  // --- SCREEN SWAPPING LOGIC --- //
  addBtn.addEventListener('click', () => {
    homeView.style.display = 'none';
    addView.style.display = 'block';
  });

  backBtn.addEventListener('click', () => {
    addView.style.display = 'none';
    homeView.style.display = 'block';
    displayNameInput.value = '';
    skinNameInput.value = '';
  });

  listBtn.addEventListener('click', () => {
    homeView.style.display = 'none';
    listView.style.display = 'block';
    refreshAccountsList();
  });

  listBackBtn.addEventListener('click', () => {
    listView.style.display = 'none';
    homeView.style.display = 'block';
  });

  // --- NEW: REMOVE PATCHES LOGIC --- //
  removePatchesBtn.addEventListener('click', async () => {
    const btnText = removePatchesBtn.querySelector('.btn-text');
    const originalText = btnText.innerText;
    
    // Change text so the user knows it's waiting for UAC
    btnText.innerText = "Awaiting Permission...";
    
    try {
      const message = await invoke('remove_patches');
      showToast(message, "success");
    } catch (error) {
      showToast(`Error: ${error}`, "error");
    }
    
    // Reset button text when done
    btnText.innerText = originalText;
  });
// --- ENTER KEY SHORTCUT --- //
[displayNameInput, skinNameInput].forEach(input => {
  input.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault(); 
      submitAccountBtn.click(); 
    }
  });
});
  // --- PATCH ACCOUNT LOGIC --- //
  submitAccountBtn.addEventListener('click', async () => {
    const displayName = displayNameInput.value;
    const skinName = skinNameInput.value;

    if (!displayName || !skinName) {
        showToast("ERROR: Please fill in both fields!", "error");
        return;
    }

    const btnText = submitAccountBtn.querySelector('.btn-text');
    btnText.innerText = "Patching System..."; // Updated to make sense for the UAC popup!

    try {
      const message = await invoke('add_new_account', { displayName: displayName, skinName: skinName });
      showToast(message, "success");
      
      btnText.innerText = "Patch Account";
      displayNameInput.value = '';
      skinNameInput.value = '';
    } catch (error) {
      showToast(`ERROR: ${error}`, "error");
      btnText.innerText = "Patch Account";
    }
  });

  // --- LIST & DELETE ACCOUNTS LOGIC --- //
  async function refreshAccountsList() {
    accountsContainer.innerHTML = '<p style="color: var(--violet);">Loading accounts...</p>';
    
    try {
      const accounts = await invoke('get_accounts');
      accountsContainer.innerHTML = ''; 
      
      if (accounts.length === 0) {
        accountsContainer.innerHTML = '<p>No accounts found.</p>';
        return;
      }

      // Draw all the cards
      accounts.forEach(acc => {
        const badgeHTML = acc.is_active ? `<div class="active-badge">ACTIVE</div>` : '';
        const activeClass = acc.is_active ? 'active-card' : '';
        
        const cardHTML = `
          <div class="account-card ${activeClass}" data-id="${acc.local_id}">
            <div class="account-info">
              <span class="account-name">${acc.username}</span>
              <span class="account-uuid">${acc.uuid}</span>
            </div>
            
            <div style="display: flex; gap: 10px; align-items: center;">
              ${badgeHTML}
              <div class="delete-btn" title="Delete Account">✕</div>
            </div>
          </div>
        `;
        
        accountsContainer.insertAdjacentHTML('beforeend', cardHTML);
      });

      // Attach Click Actions to the new cards
      const cards = accountsContainer.querySelectorAll('.account-card');
      cards.forEach(card => {
        
        // 1. CLICK TO ACTIVATE
        card.addEventListener('click', async () => {
          if (card.classList.contains('active-card')) return;

          const localId = card.getAttribute('data-id');
          try {
            await invoke('set_active_account', { localId: localId });
            
            const oldActiveCard = accountsContainer.querySelector('.active-card');
            if (oldActiveCard) {
              oldActiveCard.classList.remove('active-card');
              const oldBadge = oldActiveCard.querySelector('.active-badge');
              if (oldBadge) oldBadge.remove();
            }

            card.classList.add('active-card');
// Find the delete button, and put the badge perfectly before it!
            const targetDeleteBtn = card.querySelector('.delete-btn');
            targetDeleteBtn.insertAdjacentHTML('beforebegin', `<div class="active-badge">ACTIVE</div>`);
          } catch (error) {
            showToast(`Error: ${error}`, "error");
          }
        });

        // 2. CLICK TRASH CAN TO DELETE
        const deleteBtn = card.querySelector('.delete-btn');
        deleteBtn.addEventListener('click', async (e) => {
          e.stopPropagation(); 
          
          const localId = card.getAttribute('data-id');
          try {
            await invoke('remove_account', { localId: localId });
            showToast("Account removed", "success");
            
            // Smooth delete animation
            card.classList.add('deleting');
            setTimeout(() => {
              card.remove();
            }, 300);

          } catch (error) {
            showToast(`Error: ${error}`, "error");
          }
        });

      });

    } catch (error) {
      showToast("Error loading accounts", "error");
      accountsContainer.innerHTML = '<p>Error loading data.</p>';
    }
  }
// --- LUXURY SMOOTH SCROLLING (LENIS) --- //
  // We initialize the premium engine on our specific container
  const lenis = new Lenis({
    wrapper: accountsContainer, 
    content: accountsContainer, 
    lerp: 0.12,         // The friction/inertia (Lower = smoother/floatier)
    smoothWheel: true,  // Overrides the clunky Windows mouse wheel
    wheelMultiplier: 1  // Scroll speed
  });

  // This is the heartbeat that keeps the physics calculating 60 times a second
  function raf(time) {
    lenis.raf(time);
    requestAnimationFrame(raf);
  }
  requestAnimationFrame(raf);
});