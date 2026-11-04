################################################################################
#
# kamish
#
################################################################################

KAMISH_VERSION = 0.1.0
KAMISH_SITE = $(BR2_EXTERNAL_KAMISH_PATH)/../../
KAMISH_SITE_METHOD = local

define KAMISH_INSTALL_TARGET_CMDS
	mkdir -p $(TARGET_DIR)/opt/kamish/bin
	mkdir -p $(TARGET_DIR)/opt/kamish/dist
	mkdir -p $(TARGET_DIR)/data/kamish
	# Copy backend executable
	if [ -f $(@D)/backend/target/aarch64-unknown-linux-gnu/release/smart-speaker-backend ]; then \
		$(INSTALL) -D -m 0755 $(@D)/backend/target/aarch64-unknown-linux-gnu/release/smart-speaker-backend \
			$(TARGET_DIR)/opt/kamish/bin/smart-speaker-backend; \
	elif [ -f $(@D)/backend/target/release/smart-speaker-backend ]; then \
		$(INSTALL) -D -m 0755 $(@D)/backend/target/release/smart-speaker-backend \
			$(TARGET_DIR)/opt/kamish/bin/smart-speaker-backend; \
	fi
	# Copy frontend distribution
	if [ -d $(@D)/frontend/dist ]; then \
		cp -r $(@D)/frontend/dist/* $(TARGET_DIR)/opt/kamish/dist/; \
	fi
	# Copy migrations
	mkdir -p $(TARGET_DIR)/opt/kamish/migrations
	if [ -d $(@D)/backend/migrations ]; then \
		cp -r $(@D)/backend/migrations/* $(TARGET_DIR)/opt/kamish/migrations/; \
	fi
endef

$(eval $(generic-package))
