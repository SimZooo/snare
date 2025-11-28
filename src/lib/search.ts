export function filter_query(query: string, items: object): object {
    let individual_queries = query.split(";")
        .map((query) => query.split(":"))
        .filter((q) => q[0] !== "");

    let filtered_items = {...items};

    individual_queries.forEach(([q, v]) => {
        q = q.trim();
        v = v.trim();

        let [obj, query] = q.split(".");
        obj = obj.trim();
        query = query.trim();

        console.log(`Checking object: ${obj} for field: ${query} equals ${v}`);
        if (filtered_items[obj]) {
            console.log(filtered_items[obj])
            filtered_items[obj] = filtered_items[obj].filter((item) => item[query].toString() === v.toString());
        }
    });

    Object.keys(filtered_items).forEach((key) => {
        if (Array.isArray(filtered_items[key])) {
            filtered_items[key] = [...filtered_items[key]].sort((a, b) => {
                // Default sort key = id
                if (a.id < b.id) return -1;
                if (a.id > b.id) return 1;
                return 0;
            });
        }
    });


    return filtered_items;
}