document.addEventListener("load", () => {
    document.querySelectorAll("time").forEach(el => {
        const localDate = new Date(el.getAttribute("datetime"))
            .toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
        el.innerText = localDate;
    });
})