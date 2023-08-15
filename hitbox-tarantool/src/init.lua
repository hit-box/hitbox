local fiber = require("fiber")
local log = require("log")

box.cfg({})

local space_name = ...

box.schema.space.create(space_name, { if_not_exists = true })
box.space[space_name]:create_index("primary", {
    parts = { { 1, "string" } },
    if_not_exists = true,
})

if not _G.__hitbox_cache_fiber then
    _G.__hitbox_cache_fiber = fiber.create(function()
        fiber.name("hitbox_cache_fiber")
        while true do
            local ok, err = pcall(function()
                for _, t in box.space[space_name]:pairs() do
                    if t[2] <= fiber.time() then
                        box.space[space_name]:delete(t[1])
                    end
                end
            end)

            if not ok then
                log.error(err)
            end

            fiber.testcancel()
            fiber.sleep(1)
        end
    end)
end
