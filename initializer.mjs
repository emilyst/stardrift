// Trunk initializer for WebAssembly loading progress
export default function() {
    return {
        onStart: function() {
            console.log("Loading...");
        },

        onProgress: function({current, total}) {
            const progress = (current / total) * 100;
            console.log(`Loading: ${Math.round(progress)}%`);

            const progressBar = document.getElementById("loading-progress-bar");
            if (progressBar) {
                progressBar.style.width = progress + "%";
            }

            const percentage = document.getElementById("loading-percentage");
            if (percentage) {
                percentage.textContent = Math.round(progress) + "%";
            }
        },

        onComplete: function() {
            console.log("Loading complete");
        },

        onSuccess: function(wasm) {
            console.log("Starting...");

            const container = document.getElementById("loading-container");
            if (container) {
                container.style.opacity = "0";
                setTimeout(() => {
                    container.style.display = "none";
                }, 500);
            }
        },

        onFailure: function(error) {
            console.error("Loading failed:", error);

            const percentage = document.getElementById("loading-percentage");
            if (percentage) {
                percentage.textContent = "Error";
                percentage.style.color = "#ff4444";
            }
        }
    };
}
