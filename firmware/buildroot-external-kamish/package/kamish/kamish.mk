################################################################################
#
# kamish
#
################################################################################

KAMISH_VERSION = 0.1.0
KAMISH_SITE = $(BR2_EXTERNAL_KAMISH_PATH)/../../
KAMISH_SITE_METHOD = local

define KAMISH_INSTALL_TARGET_CMDS
	mkdir -p $(TARGET_DIR)/data
	mkdir -p $(TARGET_DIR)/opt/kamish/bin
	mkdir -p $(TARGET_DIR)/opt/kamish/dist
	mkdir -p $(TARGET_DIR)/var/lib/kamish
	mkdir -p $(TARGET_DIR)/var/log/kamish
	mkdir -p $(TARGET_DIR)/etc/kamish
	# Enable systemd units for persistent mounts and Kamish services
	mkdir -p $(TARGET_DIR)/etc/systemd/system/sysinit.target.wants
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /etc/systemd/system/kamish-mounts.service \
		$(TARGET_DIR)/etc/systemd/system/sysinit.target.wants/kamish-mounts.service
	ln -sf /etc/systemd/system/kamish-core.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/kamish-core.service
	ln -sf /etc/systemd/system/kamish-web.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/kamish-web.service
	ln -sf /etc/systemd/system/kamish.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/kamish.service
	# Copy decoupled binaries (kamish-core, kamish-web)
	for bin in kamish-core kamish-web; do \
		if [ -f $(@D)/target/aarch64-unknown-linux-gnu/release/$$bin ]; then \
			$(INSTALL) -D -m 0755 $(@D)/target/aarch64-unknown-linux-gnu/release/$$bin \
				$(TARGET_DIR)/opt/kamish/bin/$$bin; \
		elif [ -f $(@D)/target/release/$$bin ]; then \
			$(INSTALL) -D -m 0755 $(@D)/target/release/$$bin \
				$(TARGET_DIR)/opt/kamish/bin/$$bin; \
		fi; \
	done
	# Copy frontend distribution (SPA lives inside the kamish-web crate)
	if [ -d $(@D)/crates/kamish-web/frontend/dist ]; then \
		cp -r $(@D)/crates/kamish-web/frontend/dist/* $(TARGET_DIR)/opt/kamish/dist/; \
	fi
	# Copy unified SQLite database schema
	mkdir -p $(TARGET_DIR)/opt/kamish/database
	if [ -f $(@D)/database/schema.sql ]; then \
		cp $(@D)/database/schema.sql $(TARGET_DIR)/opt/kamish/database/; \
	fi
endef

$(eval $(generic-package))
