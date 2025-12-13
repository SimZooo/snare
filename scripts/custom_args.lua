function schema()
    return {
        name = "Custom Args",
        description = "Test of changing user-agent header",
        args = {
            user_agent = "String"
        }
    }
end

function on_request(req, args)
    local lines = {}
    
    -- Use gmatch with pattern that captures everything including empty strings
    for line in string.gmatch(req, "(.-)\r\n") do
        table.insert(lines, line)
    end
    
    -- Process each line
    for i, line in ipairs(lines) do
        local part = string.lower(line)
        if string.match(part, "^%s*user%-agent%s*:") then
            lines[i] = "User-Agent: " .. args.user_agent
        end
    end
    
    -- Reconstruct request with proper \r\n
    local new_req = table.concat(lines, "\r\n")
    
    -- If there's no body, ensure we end with \r\n
    if not string.match(req, "\r\n\r\n[^\r\n]") then
        new_req = new_req .. "\r\n"
    end
    
    return new_req, true
end

function on_response(res)
    return res
end